use ffmpeg_next as ffmpeg;
use ffmpeg_next::codec::context::Context as CodecContext;
use ffmpeg_next::format::{input, output};
use ffmpeg_next::media::Type;
use ffmpeg_next::software::scaling::{Context as ScalerContext, Flags};
use ffmpeg_next::util::frame::video::Video;
use image::{Rgba, RgbaImage};
use std::f32;
use std::ffi::{c_char, c_int, CStr};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

// ============================================================================
// FFI Structures
// ============================================================================

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CPoint {
    pub x: f32,
    pub y: f32,
    pub timestamp_ms: f64,
}

#[repr(C)]
pub struct CSmoothedPath {
    pub points: *mut CPoint,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VideoProcessingConfig {
    pub smoothing_alpha: f32,
    pub spring_tension: f32,
    pub spring_friction: f32,
    pub spring_mass: f32,
    pub frame_rate: i32,
}

// Optional: Progress callback type
pub type ProgressCallback = extern "C" fn(percent: f32);

// ============================================================================
// Main FFI Entry Point - Unified Video Processing
// ============================================================================

/// Process video with cursor smoothing and overlay in one call
/// Returns 0 on success, negative error codes on failure
#[no_mangle]
pub unsafe extern "C" fn process_video_with_cursor(
    input_video_path: *const c_char,
    output_video_path: *const c_char,
    cursor_sprite_path: *const c_char,
    raw_cursor_points: *const CPoint,
    raw_cursor_points_len: usize,
    config: *const VideoProcessingConfig,
    progress_callback: Option<ProgressCallback>,
) -> c_int {
    // Validate inputs
    if input_video_path.is_null()
        || output_video_path.is_null()
        || cursor_sprite_path.is_null()
        || raw_cursor_points.is_null()
        || config.is_null()
    {
        eprintln!("Error: Null pointer passed to process_video_with_cursor");
        return -1;
    }

    // Convert C strings to Rust strings
    let input_path = match unsafe { CStr::from_ptr(input_video_path) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: Invalid input_video_path UTF-8: {}", e);
            return -2;
        }
    };

    let output_path = match unsafe { CStr::from_ptr(output_video_path) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: Invalid output_video_path UTF-8: {}", e);
            return -2;
        }
    };

    let cursor_path = match unsafe { CStr::from_ptr(cursor_sprite_path) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: Invalid cursor_sprite_path UTF-8: {}", e);
            return -2;
        }
    };

    // Convert raw pointers to slices
    let cursor_points =
        unsafe { std::slice::from_raw_parts(raw_cursor_points, raw_cursor_points_len) };
    let cfg = unsafe { &*config };

    // Report initial progress
    if let Some(cb) = progress_callback {
        cb(0.0);
    }

    // Step 1: Smooth the cursor path
    let smoothed_points = match smooth_cursor_path_internal(cursor_points, cfg) {
        Ok(points) => points,
        Err(e) => {
            eprintln!("Error smoothing cursor path: {}", e);
            return -3;
        }
    };

    if let Some(cb) = progress_callback {
        cb(0.1); // 10% complete after smoothing
    }

    // Step 2: Render video with cursor overlay
    match render_video_with_overlay(
        input_path,
        cursor_path,
        output_path,
        &smoothed_points,
        progress_callback,
    ) {
        Ok(_) => {
            if let Some(cb) = progress_callback {
                cb(1.0); // 100% complete
            }
            0 // Success
        }
        Err(e) => {
            eprintln!("Error rendering video: {}", e);
            -4
        }
    }
}

// ============================================================================
// Internal Smoothing Logic
// ============================================================================

fn smooth_cursor_path_internal(
    cursor_points: &[CPoint],
    config: &VideoProcessingConfig,
) -> Result<Vec<CPoint>, String> {
    if cursor_points.len() < 4 {
        return Err("Need at least 4 cursor points for smoothing".to_string());
    }

    // Calculate number of interpolation points per segment
    let frame_counts = calculate_frames_between_points(cursor_points, config.frame_rate);

    if frame_counts.len() != cursor_points.len() - 1 {
        return Err("Frame count mismatch".to_string());
    }

    let total_expected_points: usize = frame_counts.iter().sum();
    let mut all_spline_points: Vec<CPoint> = Vec::with_capacity(total_expected_points);

    // Process each segment using Catmull-Rom splines
    let quadruple_size = 4;
    for (i, window) in cursor_points.windows(quadruple_size).enumerate() {
        let p0 = window[0];
        let p1 = window[1];
        let p2 = window[2];
        let p3 = window[3];

        let num_points = frame_counts[i + 1]; // Adjusted index
        if num_points > 0 {
            let segment_points =
                catmull_rom_spline(p0, p1, p2, p3, num_points, config.smoothing_alpha);
            all_spline_points.extend(segment_points);
        }
    }

    Ok(all_spline_points)
}

fn calculate_frames_between_points(cursor_points: &[CPoint], frame_rate: i32) -> Vec<usize> {
    let mut frame_counts = Vec::with_capacity(cursor_points.len().saturating_sub(1));

    for i in 0..cursor_points.len().saturating_sub(1) {
        let time_delta_ms = cursor_points[i + 1].timestamp_ms - cursor_points[i].timestamp_ms;
        let time_delta_seconds = time_delta_ms / 1000.0;
        let num_frames = (time_delta_seconds * frame_rate as f64).round() as usize;
        frame_counts.push(num_frames);
    }

    frame_counts
}

// ============================================================================
// Video Rendering with FFmpeg
// ============================================================================

fn render_video_with_overlay(
    input_path: &str,
    overlay_path: &str,
    output_path: &str,
    smoothed_path: &[CPoint],
    progress_callback: Option<ProgressCallback>,
) -> Result<(), Box<dyn std::error::Error>> {
    ffmpeg::init()?;

    let overlay_img = image::open(overlay_path)?.to_rgba8();

    let mut ictx = input(&Path::new(input_path))?;
    let mut octx = output(&Path::new(output_path))?;

    let input_stream = ictx
        .streams()
        .best(Type::Video)
        .ok_or("No video stream found")?;
    let video_stream_index = input_stream.index();
    let input_time_base = input_stream.time_base();
    let total_frames = input_stream.frames() as f32;

    let mut decoder = CodecContext::from_parameters(input_stream.parameters())?
        .decoder()
        .video()?;

    let frame_rate = input_stream.avg_frame_rate();
    let codec = ffmpeg::encoder::find(octx.format().codec(output_path, Type::Video))
        .ok_or("Encoder not found")?;

    // KEY: Store encoder_time_base for later use
    let encoder_time_base = ffmpeg::Rational::new(1, 60);

    let (mut encoder, ostream_index) = {
        let mut encoder_builder = CodecContext::new_with_codec(codec).encoder().video()?;

        encoder_builder.set_height(decoder.height());
        encoder_builder.set_width(decoder.width());
        encoder_builder.set_aspect_ratio(decoder.aspect_ratio());
        encoder_builder.set_format(ffmpeg::format::Pixel::YUV420P);
        encoder_builder.set_time_base(encoder_time_base);

        if frame_rate.numerator() > 0 {
            encoder_builder.set_frame_rate(Some(frame_rate));
        }

        if octx
            .format()
            .flags()
            .contains(ffmpeg::format::Flags::GLOBAL_HEADER)
        {
            encoder_builder.set_flags(ffmpeg::codec::Flags::GLOBAL_HEADER);
        }

        let encoder = encoder_builder.open()?;
        let mut ost = octx.add_stream(codec)?;
        ost.set_parameters(&encoder);

        let ostream_index = ost.index();

        (encoder, ostream_index)
    };

    octx.write_header()?;

    let mut input_to_rgb_scaler = ScalerContext::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg::format::Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR,
    )?;

    let mut rgb_to_output_scaler = ScalerContext::get(
        ffmpeg::format::Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        encoder.format(),
        encoder.width(),
        encoder.height(),
        Flags::BILINEAR,
    )?;

    let mut frame_number: i64 = 0;
    let mut processed_frames = 0;

    // KEY FIX: Use encoder_time_base (1/60) directly, not output_time_base
    // Calculate PTS increment: for 60 fps at time_base 1/60, increment = 1
    let pts_increment = if frame_rate.numerator() > 0 && frame_rate.denominator() > 0 {
        // frames per second = numerator / denominator
        // time units per frame = time_base / frame_rate
        // = (1/60) / (60/1) = 1/60 * 1/60 = 1
        (encoder_time_base.denominator() as i64 * frame_rate.denominator() as i64)
            / (encoder_time_base.numerator() as i64 * frame_rate.numerator() as i64)
    } else {
        1 // Fallback to 1 time unit per frame
    };

    let mut next_pts: i64 = 0;

    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet)?;

            let mut decoded_frame = Video::empty();
            while decoder.receive_frame(&mut decoded_frame).is_ok() {
                let original_timestamp = decoded_frame.timestamp();
                let timestamp_ms = if let Some(ts) = original_timestamp {
                    ts as f64 * 1000.0 * f64::from(input_time_base)
                } else {
                    0.0
                };

                let mut rgb_frame = Video::empty();
                input_to_rgb_scaler.run(&decoded_frame, &mut rgb_frame)?;
                rgb_frame.set_pts(Some(frame_number));

                if let Some(pos) = find_position_for_timestamp(smoothed_path, timestamp_ms) {
                    overlay_image_on_rgb_frame(
                        &mut rgb_frame,
                        &overlay_img,
                        pos.x as i32,
                        pos.y as i32,
                    );
                }

                let mut output_frame = Video::empty();
                rgb_to_output_scaler.run(&rgb_frame, &mut output_frame)?;
                output_frame.set_pts(Some(frame_number));

                encoder.send_frame(&output_frame)?;

                let mut encoded_packet = ffmpeg::Packet::empty();
                while encoder.receive_packet(&mut encoded_packet).is_ok() {
                    encoded_packet.set_stream(ostream_index);

                    // Set timestamps directly in encoder_time_base
                    encoded_packet.set_pts(Some(next_pts));
                    encoded_packet.set_dts(Some(next_pts));

                    encoded_packet.write_interleaved(&mut octx)?;
                    next_pts += pts_increment;
                }

                frame_number += 1;
                processed_frames += 1;

                if let Some(cb) = progress_callback {
                    if total_frames > 0.0 {
                        let progress = 0.1 + (0.9 * (processed_frames as f32 / total_frames));
                        cb(progress.min(0.99));
                    }
                }
            }
        }
    }

    // Flush decoder
    decoder.send_eof()?;
    let mut decoded_frame = Video::empty();
    while decoder.receive_frame(&mut decoded_frame).is_ok() {
        let original_timestamp = decoded_frame.timestamp();
        let timestamp_ms = if let Some(ts) = original_timestamp {
            ts as f64 * 1000.0 * f64::from(input_time_base)
        } else {
            0.0
        };

        let mut rgb_frame = Video::empty();
        input_to_rgb_scaler.run(&decoded_frame, &mut rgb_frame)?;
        rgb_frame.set_pts(Some(frame_number));

        if let Some(pos) = find_position_for_timestamp(smoothed_path, timestamp_ms) {
            overlay_image_on_rgb_frame(&mut rgb_frame, &overlay_img, pos.x as i32, pos.y as i32);
        }

        let mut output_frame = Video::empty();
        rgb_to_output_scaler.run(&rgb_frame, &mut output_frame)?;
        output_frame.set_pts(Some(frame_number));

        encoder.send_frame(&output_frame)?;

        let mut encoded_packet = ffmpeg::Packet::empty();
        while encoder.receive_packet(&mut encoded_packet).is_ok() {
            encoded_packet.set_stream(ostream_index);
            encoded_packet.set_pts(Some(next_pts));
            encoded_packet.set_dts(Some(next_pts));
            encoded_packet.write_interleaved(&mut octx)?;
            next_pts += pts_increment;
        }

        frame_number += 1;
    }

    // Flush encoder
    encoder.send_eof()?;
    let mut encoded_packet = ffmpeg::Packet::empty();
    while encoder.receive_packet(&mut encoded_packet).is_ok() {
        encoded_packet.set_stream(ostream_index);
        encoded_packet.set_pts(Some(next_pts));
        encoded_packet.set_dts(Some(next_pts));
        encoded_packet.write_interleaved(&mut octx)?;
        next_pts += pts_increment;
    }

    octx.write_trailer()?;
    Ok(())
}

// New function for RGB24 overlay
fn overlay_image_on_rgb_frame(frame: &mut Video, overlay: &RgbaImage, x_pos: i32, y_pos: i32) {
    let frame_w = frame.width() as i32;
    let frame_h = frame.height() as i32;
    let stride = frame.stride(0);
    let (overlay_w, overlay_h) = overlay.dimensions();

    let frame_data = frame.data_mut(0);

    for y_overlay in 0..overlay_h {
        for x_overlay in 0..overlay_w {
            let x_frame = x_pos + x_overlay as i32;
            let y_frame = y_pos + y_overlay as i32;

            if x_frame >= 0 && x_frame < frame_w && y_frame >= 0 && y_frame < frame_h {
                let pixel_overlay = overlay.get_pixel(x_overlay, y_overlay);
                let Rgba([r, g, b, a]) = *pixel_overlay;

                // Simple alpha blending
                if a > 0 {
                    let frame_idx = (y_frame as usize * stride) + (x_frame as usize * 3);
                    if frame_idx + 2 < frame_data.len() {
                        if a == 255 {
                            // Fully opaque - direct copy
                            frame_data[frame_idx] = r;
                            frame_data[frame_idx + 1] = g;
                            frame_data[frame_idx + 2] = b;
                        } else {
                            // Alpha blending
                            let alpha_f = a as f32 / 255.0;
                            let inv_alpha = 1.0 - alpha_f;

                            frame_data[frame_idx] = (r as f32 * alpha_f
                                + frame_data[frame_idx] as f32 * inv_alpha)
                                as u8;
                            frame_data[frame_idx + 1] = (g as f32 * alpha_f
                                + frame_data[frame_idx + 1] as f32 * inv_alpha)
                                as u8;
                            frame_data[frame_idx + 2] = (b as f32 * alpha_f
                                + frame_data[frame_idx + 2] as f32 * inv_alpha)
                                as u8;
                        }
                    }
                }
            }
        }
    }
}

fn find_position_for_timestamp(path: &[CPoint], timestamp_ms: f64) -> Option<CPoint> {
    if path.is_empty() {
        return None;
    }

    if path.len() == 1 {
        return path.first().copied();
    }

    // Find the segment containing this timestamp
    if let Some(index) = path
        .windows(2)
        .position(|w| timestamp_ms >= w[0].timestamp_ms && timestamp_ms <= w[1].timestamp_ms)
    {
        let p1 = &path[index];
        let p2 = &path[index + 1];
        let duration = p2.timestamp_ms - p1.timestamp_ms;

        let t = if duration > 0.0 {
            (timestamp_ms - p1.timestamp_ms) / duration
        } else {
            0.0
        };

        let x = p1.x as f64 + t * (p2.x as f64 - p1.x as f64);
        let y = p1.y as f64 + t * (p2.y as f64 - p1.y as f64);

        return Some(CPoint {
            x: x as f32,
            y: y as f32,
            timestamp_ms,
        });
    }

    // Return closest point if timestamp is outside range
    if timestamp_ms < path[0].timestamp_ms {
        path.first().copied()
    } else {
        path.last().copied()
    }
}

// ============================================================================
// Catmull-Rom Spline Implementation
// ============================================================================

fn catmull_rom_spline(
    p0: CPoint,
    p1: CPoint,
    p2: CPoint,
    p3: CPoint,
    num_points: usize,
    alpha: f32,
) -> Vec<CPoint> {
    let t_0: f32 = 0.0;
    let t_1 = calculate_t_j(t_0, &p0, &p1, alpha);
    let t_2 = calculate_t_j(t_1, &p1, &p2, alpha);
    let t_3 = calculate_t_j(t_2, &p2, &p3, alpha);

    let t_values = linspace(t_1, t_2, num_points);
    let mut points = Vec::with_capacity(num_points);

    for &t in &t_values {
        let a_1 = interpolate_points(t_1, t_0, t, &p0, &p1);
        let a_2 = interpolate_points(t_2, t_1, t, &p1, &p2);
        let a_3 = interpolate_points(t_3, t_2, t, &p2, &p3);
        let b_1 = interpolate_points(t_2, t_0, t, &a_1, &a_2);
        let b_2 = interpolate_points(t_3, t_1, t, &a_2, &a_3);
        let final_point = interpolate_points(t_2, t_1, t, &b_1, &b_2);
        points.push(final_point);
    }
    points
}

fn interpolate_points(
    t_end: f32,
    t_start: f32,
    t: f32,
    p_start: &CPoint,
    p_end: &CPoint,
) -> CPoint {
    let (weight1, weight2) = if (t_end - t_start).abs() < f32::EPSILON {
        if t <= t_start {
            (1.0, 0.0)
        } else {
            (0.0, 1.0)
        }
    } else {
        (
            (t_end - t) / (t_end - t_start),
            (t - t_start) / (t_end - t_start),
        )
    };

    CPoint {
        x: weight1 * p_start.x + weight2 * p_end.x,
        y: weight1 * p_start.y + weight2 * p_end.y,
        timestamp_ms: (weight1 as f64) * p_start.timestamp_ms
            + (weight2 as f64) * p_end.timestamp_ms,
    }
}

fn calculate_t_j(t_i: f32, p_i: &CPoint, p_j: &CPoint, alpha: f32) -> f32 {
    let dx = p_j.x - p_i.x;
    let dy = p_j.y - p_i.y;
    t_i + (dx.powi(2) + dy.powi(2)).sqrt().powf(alpha)
}

fn linspace(start: f32, end: f32, num_points: usize) -> Vec<f32> {
    if num_points == 0 {
        return Vec::new();
    }
    if num_points == 1 {
        return vec![start];
    }
    let step = (end - start) / (num_points - 1) as f32;
    (0..num_points).map(|i| start + (i as f32) * step).collect()
}

// ============================================================================
// Legacy FFI Functions (for backward compatibility)
// ============================================================================

#[no_mangle]
pub unsafe extern "C" fn smooth_cursor_path(
    raw_points_ptr: *const CPoint,
    raw_points_len: usize,
    points_per_segment_ptr: *const i64,
    points_per_segment_len: usize,
    alpha: f32,
    _tension: f32,
    _friction: f32,
    _mass: f32,
) -> CSmoothedPath {
    if raw_points_ptr.is_null() || points_per_segment_ptr.is_null() {
        return CSmoothedPath {
            points: std::ptr::null_mut(),
            len: 0,
        };
    }

    // SAFETY: Caller guarantees these point to valid arrays
    let points_slice: &[CPoint] = std::slice::from_raw_parts(raw_points_ptr, raw_points_len);
    let frame_amount_slice: &[i64] =
        std::slice::from_raw_parts(points_per_segment_ptr, points_per_segment_len);

    if points_slice.is_empty() || frame_amount_slice.is_empty() {
        return CSmoothedPath {
            points: std::ptr::null_mut(),
            len: 0,
        };
    }

    let quadruple_size: usize = 4;
    let num_segments = points_slice.len().saturating_sub(quadruple_size - 1);

    if num_segments == 0 || frame_amount_slice.len() != num_segments {
        return CSmoothedPath {
            points: std::ptr::null_mut(),
            len: 0,
        };
    }

    let total_expected_points: usize = frame_amount_slice.iter().map(|&x| x as usize).sum();
    let mut all_spline_points: Vec<CPoint> = Vec::with_capacity(total_expected_points);

    for (i, window) in points_slice.windows(quadruple_size).enumerate() {
        let p0 = window[0];
        let p1 = window[1];
        let p2 = window[2];
        let p3 = window[3];

        let num_points_for_this_segment = frame_amount_slice[i] as usize;
        if num_points_for_this_segment > 0 {
            let segment_points =
                catmull_rom_spline(p0, p1, p2, p3, num_points_for_this_segment, alpha);
            all_spline_points.extend(segment_points);
        }
    }

    all_spline_points.shrink_to_fit();
    let len = all_spline_points.len();
    let ptr = all_spline_points.as_mut_ptr();
    std::mem::forget(all_spline_points);

    CSmoothedPath { points: ptr, len }
}

#[no_mangle]
pub unsafe extern "C" fn free_smoothed_path(path: CSmoothedPath) {
    if !path.points.is_null() && path.len > 0 {
        // SAFETY: This path was created by smooth_cursor_path via Vec::as_mut_ptr + forget
        let _ = Vec::from_raw_parts(path.points, path.len, path.len);
        // Vec is automatically dropped here, freeing the memory
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

#[allow(dead_code)]
fn export_points_to_csv(filename: &str, points: &[CPoint]) -> std::io::Result<()> {
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "x,y,timestamp_ms")?;
    for p in points {
        writeln!(writer, "{},{},{}", p.x, p.y, p.timestamp_ms)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
