use ffmpeg_next as ffmpeg;
use ffmpeg_next::codec::context::Context as CodecContext;
use ffmpeg_next::format::{input, output};
use ffmpeg_next::media::Type;
use ffmpeg_next::software::scaling::{Context as ScalerContext, Flags};
use ffmpeg_next::util::frame::video::Video;
use image::{Rgba, RgbaImage};
use std::f32;
use std::ffi::{c_char, CStr};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

// Struct Definitions

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

// FFI Entry Point & Memory Management

fn _render_video(
    input_path: &str,
    overlay_path: &str,
    output_path: &str,
    path_points: &[CPoint],
) -> Result<(), Box<dyn std::error::Error>> {
    ffmpeg::init()?;

    let overlay_img = image::open(overlay_path)?.to_rgba8();

    let mut ictx = input(&Path::new(input_path))?;
    let mut octx = output(&Path::new(output_path))?;

    // --- Input Setup ---
    let input_stream = ictx
        .streams()
        .best(Type::Video)
        .ok_or(ffmpeg::Error::StreamNotFound)?;
    let video_stream_index = input_stream.index();
    let input_time_base = input_stream.time_base();

    let mut decoder = CodecContext::from_parameters(input_stream.parameters())?
        .decoder()
        .video()?;

    // --- Output and Encoder Setup ---
    let (mut encoder, output_time_base, ostream_index) = {
        let codec = ffmpeg::encoder::find(octx.format().codec(output_path, Type::Video))
            .ok_or(ffmpeg::Error::EncoderNotFound)?;

        let mut encoder_builder = CodecContext::new_with_codec(codec).encoder().video()?;
        encoder_builder.set_height(decoder.height());
        encoder_builder.set_width(decoder.width());
        //  Use the input stream's time_base for the encoder setup.
        encoder_builder.set_time_base(input_time_base);
        encoder_builder.set_format(decoder.format());

        if octx
            .format()
            .flags()
            .contains(ffmpeg::format::Flags::GLOBAL_HEADER)
        {
            encoder_builder.set_flags(ffmpeg::codec::Flags::GLOBAL_HEADER);
        }

        let encoder = encoder_builder.open_as(codec)?;

        // We create the stream, extract its necessary properties (index and time_base),
        // and then let the `ostream` object go out of scope. This releases the mutable
        // borrow on `octx` before the main loop begins.
        let mut ostream = octx.add_stream(encoder.codec())?;
        ostream.set_parameters(&encoder);
        let output_time_base = ostream.time_base();
        let ostream_index = ostream.index();

        (encoder, output_time_base, ostream_index)
    };

    octx.write_header()?;

    // --- Scaler Setup ---
    let mut scaler = ScalerContext::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        encoder.format(),
        encoder.width(),
        encoder.height(),
        Flags::BILINEAR,
    )?;

    // --- Processing Loop ---
    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet)?;

            let mut decoded_frame = Video::empty();
            while decoder.receive_frame(&mut decoded_frame).is_ok() {
                let timestamp = decoded_frame.timestamp().unwrap_or(0);
                let timestamp_ms = timestamp as f64 * 1000.0 * f64::from(input_time_base);

                if let Some(pos) = find_position_for_timestamp(path_points, timestamp_ms) {
                    overlay_image_on_frame(
                        &mut decoded_frame,
                        &overlay_img,
                        pos.x as i32,
                        pos.y as i32,
                    );
                }

                let mut scaled_frame = Video::empty();
                scaler.run(&decoded_frame, &mut scaled_frame)?;
                // Propagate timestamp to the scaled frame
                scaled_frame.set_pts(decoded_frame.pts());

                encoder.send_frame(&scaled_frame)?;

                let mut encoded_packet = ffmpeg::Packet::empty();
                while encoder.receive_packet(&mut encoded_packet).is_ok() {
                    encoded_packet.set_stream(ostream_index);
                    encoded_packet.rescale_ts(input_time_base, output_time_base);
                    encoded_packet.write_interleaved(&mut octx)?; // This mutable borrow is now safe
                }
            }
        }
    }

    // --- Flush Encoder and Finalize Output ---
    encoder.send_eof()?;
    let mut encoded_packet = ffmpeg::Packet::empty();
    while encoder.receive_packet(&mut encoded_packet).is_ok() {
        encoded_packet.set_stream(ostream_index);
        encoded_packet.rescale_ts(input_time_base, output_time_base);
        encoded_packet.write_interleaved(&mut octx)?;
    }

    octx.write_trailer()?;
    Ok(())
}

// --- Video Frame Helpers (Corrected) ---

fn overlay_image_on_frame(frame: &mut Video, overlay: &RgbaImage, x_pos: i32, y_pos: i32) {
    // Perform all immutable borrows *before* the mutable borrow.
    // Read properties into local variables.
    let frame_w = frame.width() as i32;
    let frame_h = frame.height() as i32;
    let stride = frame.stride(0) as usize;
    let (overlay_w, overlay_h) = overlay.dimensions();

    //  create the mutable borrow. It does not conflict with any other borrows.
    let frame_data = frame.data_mut(0);

    for y_overlay in 0..overlay_h {
        for x_overlay in 0..overlay_w {
            let x_frame = x_pos + x_overlay as i32;
            let y_frame = y_pos + y_overlay as i32;

            if x_frame >= 0 && x_frame < frame_w && y_frame >= 0 && y_frame < frame_h {
                let pixel_overlay = overlay.get_pixel(x_overlay, y_overlay);
                let Rgba([r, g, b, a]) = *pixel_overlay;

                if a > 0 {
                    // Assuming RGB24 format for this example. Be cautious if your video
                    // format is different (e.g., YUV). This logic works for formats where
                    // pixel data is stored in a simple RGB array.
                    let frame_idx = (y_frame as usize * stride) + (x_frame as usize * 3);
                    if frame_idx + 2 < frame_data.len() {
                        frame_data[frame_idx] = r;
                        frame_data[frame_idx + 1] = g;
                        frame_data[frame_idx + 2] = b;
                    }
                }
            }
        }
    }
}
// --- Video Frame Helpers ---

fn find_position_for_timestamp(path: &[CPoint], timestamp_ms: f64) -> Option<CPoint> {
    if path.len() < 2 {
        return path.first().copied();
    }

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

    if timestamp_ms < path[0].timestamp_ms {
        path.first().copied()
    } else {
        path.last().copied()
    }
}

// --- Path Smoothing & Interpolation Logic ---

#[no_mangle]
pub extern "C" fn smooth_cursor_path(
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

    let points_slice: &[CPoint] =
        unsafe { std::slice::from_raw_parts(raw_points_ptr, raw_points_len) };
    let frame_amount_slice: &[i64] =
        unsafe { std::slice::from_raw_parts(points_per_segment_ptr, points_per_segment_len) };

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
    std::mem::forget(all_spline_points); // Prevent Rust from dropping the memory

    CSmoothedPath { points: ptr, len }
}

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

// --- Utility Functions ---

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
