use ffmpeg_next as ffmpeg;
use ffmpeg_next::codec::context::Context as CodecContext;
use ffmpeg_next::format::{input, output};
use ffmpeg_next::media::Type;
use ffmpeg_next::software::scaling::{Context as ScalerContext, Flags};
use ffmpeg_next::util::frame::video::Video;
use ffmpeg_next::util::mathematics::rescale::Rescale;
use image::{Rgba, RgbaImage};
use log::{debug, info, trace, warn};
use std::f32;
use std::ffi::{c_char, c_int, CStr};
use std::sync::OnceLock;

mod constants;
mod types;
mod utils;

#[cfg(test)]
mod tests;

use crate::constants::*;
#[cfg(feature = "legacy-api")]
use crate::types::CSmoothedPath;
use crate::types::{CPoint, PathPoint, ProgressCallback, VideoProcessingConfig};
use crate::utils::{ensure_repo_output_dir, export_points_to_csv};
use std::path::Path;

// ============================================================================
// FFmpeg Initialization (thread-safe, single call)
// ============================================================================

static FFMPEG_INIT: OnceLock<bool> = OnceLock::new();

/// Ensures FFmpeg is initialized exactly once across all threads.
/// Returns Ok(()) on success, Err on failure.
fn ensure_ffmpeg_init() -> Result<(), ffmpeg::Error> {
    let initialized = FFMPEG_INIT.get_or_init(|| match ffmpeg::init() {
        Ok(()) => true,
        Err(e) => {
            warn!("FFmpeg initialization failed: {}", e);
            false
        }
    });
    if *initialized {
        Ok(())
    } else {
        Err(ffmpeg::Error::Bug)
    }
}

// ============================================================================
// Spring Parameter Derivation from Perceptual Inputs
// ============================================================================

/// Derives physics-based spring parameters from user-friendly perceptual inputs.
///
/// # Parameters
/// - `responsiveness`: 0.0 (slow/floaty) to 1.0 (snappy/immediate)
/// - `smoothness`: 0.0 (slight overshoot) to 1.0 (no overshoot, very smooth)
///
/// # Returns
/// Tuple of (stiffness k, damping c, mass m) for spring physics simulation.
pub(crate) fn derive_spring_params(responsiveness: f32, smoothness: f32) -> (f64, f64, f64) {
    let responsiveness = responsiveness.clamp(0.0, 1.0) as f64;
    let smoothness = smoothness.clamp(0.0, 1.0) as f64;

    // Map responsiveness to settling time (inverse relationship)
    // responsiveness=0 -> slow (400ms), responsiveness=1 -> fast (60ms)
    let settling_time = RESPONSIVENESS_MIN_SETTLING_SEC
        + (1.0 - responsiveness)
            * (RESPONSIVENESS_MIN_SETTLING_SEC - RESPONSIVENESS_MAX_SETTLING_SEC);
    let settling_time = settling_time.clamp(
        RESPONSIVENESS_MAX_SETTLING_SEC,
        RESPONSIVENESS_MIN_SETTLING_SEC,
    );

    // Map smoothness to damping ratio
    // smoothness=0 -> slight overshoot (zeta=0.7), smoothness=1 -> overdamped (zeta=1.5)
    let zeta =
        SMOOTHNESS_MIN_DAMPING + smoothness * (SMOOTHNESS_MAX_DAMPING - SMOOTHNESS_MIN_DAMPING);

    // Use standard mass
    let m = SPRING_MASS_DEFAULT;

    // Derive natural frequency from settling time: T_s ≈ 4/(ζ·ωn)
    let omega_n = 4.0 / (zeta * settling_time);

    // Spring stiffness: k = ωn² · m
    let k = omega_n * omega_n * m;

    // Damping coefficient: c = 2·ζ·ωn·m
    let c = 2.0 * zeta * omega_n * m;

    debug!(
        "derive_spring_params: responsiveness={:.2} smoothness={:.2} -> Ts={:.3}s zeta={:.2} k={:.1} c={:.1}",
        responsiveness, smoothness, settling_time, zeta, k, c
    );

    (k, c, m)
}

// ============================================================================
// Timestamp Alignment State (robust multi-frame alignment)
// ============================================================================

/// State for computing robust timestamp alignment using median of first N frames.
struct AlignmentState {
    samples: Vec<f64>,
    max_samples: usize,
    max_window_ms: f64,
    first_frame_ms: Option<f64>,
    computed_offset: Option<f64>,
}

impl AlignmentState {
    fn new() -> Self {
        Self {
            samples: Vec::with_capacity(ALIGNMENT_SAMPLE_COUNT),
            max_samples: ALIGNMENT_SAMPLE_COUNT,
            max_window_ms: ALIGNMENT_MAX_WINDOW_MS,
            first_frame_ms: None,
            computed_offset: None,
        }
    }

    /// Add a frame timestamp sample. Returns true if alignment is ready.
    fn add_sample(&mut self, frame_ms: f64, path_start_ms: f64) -> bool {
        if self.computed_offset.is_some() {
            return true;
        }

        if self.first_frame_ms.is_none() {
            self.first_frame_ms = Some(frame_ms);
        }

        // Check if we've exceeded the time window
        let elapsed = frame_ms - self.first_frame_ms.unwrap_or(frame_ms);
        if elapsed > self.max_window_ms && !self.samples.is_empty() {
            self.compute_offset(path_start_ms);
            return true;
        }

        // Add sample: offset = path_start - frame_ms
        self.samples.push(path_start_ms - frame_ms);

        if self.samples.len() >= self.max_samples {
            self.compute_offset(path_start_ms);
            return true;
        }

        false
    }

    /// Compute the median offset from collected samples.
    fn compute_offset(&mut self, _path_start_ms: f64) {
        if self.samples.is_empty() {
            self.computed_offset = Some(0.0);
            return;
        }

        // Sort and take median
        let mut sorted = self.samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let median = if sorted.len() % 2 == 0 {
            let mid = sorted.len() / 2;
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[sorted.len() / 2]
        };

        info!(
            "Alignment computed: median offset={:.3}ms from {} samples",
            median,
            self.samples.len()
        );

        self.computed_offset = Some(median);
    }

    /// Get the computed offset, or compute it now if enough samples.
    fn get_offset(&mut self, path_start_ms: f64) -> f64 {
        if self.computed_offset.is_none() && !self.samples.is_empty() {
            self.compute_offset(path_start_ms);
        }
        self.computed_offset.unwrap_or(0.0)
    }
}

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
        warn!("Null pointer passed to process_video_with_cursor");
        return -1;
    }

    // Convert C strings to Rust strings
    let input_path = match CStr::from_ptr(input_video_path).to_str() {
        Ok(s) => s,
        Err(e) => {
            warn!("Invalid input_video_path UTF-8: {}", e);
            return -2;
        }
    };

    let output_path = match CStr::from_ptr(output_video_path).to_str() {
        Ok(s) => s,
        Err(e) => {
            warn!("Invalid output_video_path UTF-8: {}", e);
            return -2;
        }
    };

    let cursor_path = match CStr::from_ptr(cursor_sprite_path).to_str() {
        Ok(s) => s,
        Err(e) => {
            warn!("Invalid cursor_sprite_path UTF-8: {}", e);
            return -2;
        }
    };

    // Convert raw pointers to slices
    let cursor_points = std::slice::from_raw_parts(raw_cursor_points, raw_cursor_points_len);
    let cfg = &*config;

    // Report initial progress
    if let Some(cb) = progress_callback {
        cb(0.0);
    }

    // Step 1: Inspect input to derive video fps and duration for time-accurate smoothing
    let (video_fps_num, video_fps_den, video_duration_ms) =
        match (|| -> Result<(i32, i32, f64), Box<dyn std::error::Error>> {
            ensure_ffmpeg_init()?;
            let ictx = input(&Path::new(input_path))?;
            let stream = ictx
                .streams()
                .best(Type::Video)
                .ok_or("No video stream found")?;
            let fr = stream.avg_frame_rate();
            let fps_num = fr.numerator();
            let fps_den = fr.denominator();
            let time_base = stream.time_base();
            // Prefer duration from stream if available (>0); else estimate
            let dur_pts: i64 = stream.duration();
            let dur_ms = if dur_pts > 0 {
                dur_pts.rescale(time_base, ffmpeg::Rational(1, 1000)) as f64
            } else {
                let frames = stream.frames() as f64;
                if fps_num > 0 && fps_den > 0 {
                    1000.0 * frames * (fps_den as f64 / fps_num as f64)
                } else {
                    // fallback to cursor span when fps unknown
                    let (first_ms, last_ms) = (
                        cursor_points.first().map(|p| p.timestamp_ms).unwrap_or(0.0),
                        cursor_points.last().map(|p| p.timestamp_ms).unwrap_or(0.0),
                    );
                    (last_ms - first_ms).max(0.0)
                }
            };
            Ok((fps_num as i32, fps_den as i32, dur_ms))
        })() {
            Ok(t) => t,
            Err(e) => {
                warn!(
                    "Failed to read video info: {} - falling back to config fps and cursor duration",
                    e
                );
                let fps = cfg.frame_rate;
                let (first_ms, last_ms) = (
                    cursor_points.first().map(|p| p.timestamp_ms).unwrap_or(0.0),
                    cursor_points.last().map(|p| p.timestamp_ms).unwrap_or(0.0),
                );
                (fps, 1, (last_ms - first_ms).max(0.0))
            }
        };

    info!(
        "Video: fps={}/{} (~{:.2} fps) duration~{:.1}ms",
        video_fps_num,
        video_fps_den,
        if video_fps_den != 0 {
            video_fps_num as f64 / video_fps_den as f64
        } else {
            0.0
        },
        video_duration_ms
    );

    // Step 2: Smooth the cursor path using video fps and duration
    let smoothed_points = match smooth_cursor_path_with_params(
        cursor_points,
        cfg,
        if video_fps_den != 0 {
            (video_fps_num as f64 / video_fps_den as f64).round() as i32
        } else {
            cfg.frame_rate
        },
        video_duration_ms,
    ) {
        Ok(points) => points,
        Err(e) => {
            warn!("Error smoothing cursor path: {}", e);
            return -3;
        }
    };

    if let Some(cb) = progress_callback {
        cb(0.1); // 10% complete after smoothing
    }

    // Step 3: Render video with cursor overlay
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
            warn!("Error rendering video: {}", e);
            -4
        }
    }
}

// ============================================================================
// Internal Smoothing Logic
// ============================================================================

fn smooth_cursor_path_with_params(
    cursor_points: &[CPoint],
    config: &VideoProcessingConfig,
    target_fps: i32,
    _desired_total_ms: f64,
) -> Result<Vec<PathPoint>, String> {
    if cursor_points.len() < 4 {
        return Err("Need at least 4 cursor points for smoothing".to_string());
    }

    let frame_counts = calculate_frames_between_points(cursor_points, target_fps);
    if frame_counts.len() != cursor_points.len() - 1 {
        return Err("Frame count mismatch".to_string());
    }

    // Duplicate endpoints to cover [p0,p1] and [p_{n-2},p_{n-1}]
    let n = cursor_points.len();
    let mut ext: Vec<CPoint> = Vec::with_capacity(n + 2);
    ext.push(cursor_points[0]);
    ext.extend_from_slice(cursor_points);
    ext.push(cursor_points[n - 1]);

    let total_expected_points: usize = frame_counts.iter().sum();
    let mut all_spline_points: Vec<CPoint> = Vec::with_capacity(total_expected_points);

    debug!(
        "Spline: raw_points={}, expected_samples={}, fps={}",
        n, total_expected_points, config.frame_rate
    );

    for seg in 0..(n - 1) {
        let p0 = ext[seg];
        let p1 = ext[seg + 1];
        let p2 = ext[seg + 2];
        let p3 = ext[seg + 3];

        let num_points = frame_counts[seg];
        if num_points == 0 {
            continue;
        }

        let segment_points = catmull_rom_spline(p0, p1, p2, p3, num_points, config.smoothing_alpha);

        if seg < 2 {
            let t0 = segment_points
                .first()
                .map(|p| p.timestamp_ms)
                .unwrap_or(0.0);
            let t1 = segment_points.last().map(|p| p.timestamp_ms).unwrap_or(0.0);
            trace!(
                "Segment {}: {} points, span=[{:.3},{:.3}]ms raw=[{:.3},{:.3}]ms",
                seg,
                num_points,
                t0,
                t1,
                p1.timestamp_ms,
                p2.timestamp_ms
            );
        }

        // Avoid duplicating the first point of each segment after the first
        if seg > 0 && !segment_points.is_empty() {
            all_spline_points.extend(segment_points.into_iter().skip(1));
        } else {
            all_spline_points.extend(segment_points);
        }
    }

    debug!("Spline output: {} points", all_spline_points.len());

    // Debug: export raw and spline points to CSV for inspection
    if log::log_enabled!(log::Level::Trace) {
        if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            let suffix = now.as_millis();
            let out_dir = ensure_repo_output_dir();
            let _ = export_points_to_csv(
                &out_dir
                    .join(format!("debug_raw_points_{}.csv", suffix))
                    .to_string_lossy(),
                cursor_points,
            );
            let _ = export_points_to_csv(
                &out_dir
                    .join(format!("debug_spline_points_{}.csv", suffix))
                    .to_string_lossy(),
                &all_spline_points,
            );
        }
    }

    // Generate arc-length based targets with trapezoidal speed profile at frame cadence
    let base_ts = cursor_points.first().map(|p| p.timestamp_ms).unwrap_or(0.0);
    let targets = generate_targets_with_click_constraints(
        cursor_points,
        &all_spline_points,
        base_ts,
        target_fps,
    );

    debug!(
        "Targets: count={} t0={:.3}ms tN={:.3}ms duration={:.3}ms",
        targets.len(),
        targets.first().map(|p| p.timestamp_ms).unwrap_or(0.0),
        targets.last().map(|p| p.timestamp_ms).unwrap_or(0.0),
        targets.last().map(|p| p.timestamp_ms).unwrap_or(0.0)
            - targets.first().map(|p| p.timestamp_ms).unwrap_or(0.0)
    );

    if log::log_enabled!(log::Level::Trace) {
        if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            let suffix = now.as_millis();
            let out_dir = ensure_repo_output_dir();
            let _ = export_points_to_csv(
                &out_dir
                    .join(format!("debug_targets_{}.csv", suffix))
                    .to_string_lossy(),
                &targets,
            );
        }
    }

    // Apply physics-based spring filter to follow the shaped targets
    let filtered = spring_follow_path(&targets, config);

    if let Some((f0, f_n)) = filtered.first().zip(filtered.last()) {
        debug!(
            "Spring: count={} t0={:.3}ms tN={:.3}ms duration={:.3}ms first=({:.1},{:.1}) last=({:.1},{:.1})",
            filtered.len(),
            f0.timestamp_ms,
            f_n.timestamp_ms,
            f_n.timestamp_ms - f0.timestamp_ms,
            f0.x,
            f0.y,
            f_n.x,
            f_n.y
        );
    }

    if log::log_enabled!(log::Level::Trace) {
        if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            let suffix = now.as_millis();
            let out_dir = ensure_repo_output_dir();
            // Convert PathPoints to CPoints for CSV export
            let cpoints: Vec<CPoint> = filtered.iter().map(|p| p.to_cpoint()).collect();
            let _ = export_points_to_csv(
                &out_dir
                    .join(format!("debug_spring_{}.csv", suffix))
                    .to_string_lossy(),
                &cpoints,
            );
        }
    }

    Ok(filtered)
}

fn calculate_frames_between_points(cursor_points: &[CPoint], frame_rate: i32) -> Vec<usize> {
    let mut frame_counts = Vec::with_capacity(cursor_points.len().saturating_sub(1));
    for i in 0..cursor_points.len().saturating_sub(1) {
        let time_delta_ms = cursor_points[i + 1].timestamp_ms - cursor_points[i].timestamp_ms;
        let time_delta_seconds = time_delta_ms / 1000.0;
        let num_frames = ((time_delta_seconds * frame_rate as f64).round() as usize).max(1);
        trace!(
            "Segment {}: dt={:.3}ms fps={} frames={}",
            i,
            time_delta_ms,
            frame_rate,
            num_frames
        );
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
    smoothed_path: &[PathPoint],
    progress_callback: Option<ProgressCallback>,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_ffmpeg_init()?;

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

    // Match encoder time base to input for simpler reasoning
    let encoder_time_base = input_time_base;

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

    // Per-frame duration in encoder_time_base
    let frame_duration_pts: i64 = if frame_rate.numerator() > 0 && frame_rate.denominator() > 0 {
        1_i64.rescale(
            ffmpeg::Rational(frame_rate.denominator(), frame_rate.numerator()),
            encoder_time_base,
        )
    } else {
        1
    };

    let mut cur_pts: i64 = 0;

    // Robust timestamp alignment using median of first N frames
    let mut alignment = AlignmentState::new();
    let path_start_ms = smoothed_path.first().map(|p| p.timestamp_ms).unwrap_or(0.0);

    let fallback_ms_from_frame_index = |idx: i64| -> f64 {
        if frame_rate.numerator() > 0 && frame_rate.denominator() > 0 {
            (idx as f64)
                * (1000.0 * frame_rate.denominator() as f64 / frame_rate.numerator() as f64)
        } else {
            (idx as f64) * (1000.0 / 60.0)
        }
    };

    info!(
        "Render: encoder_tb={}/{} fps={}/{}",
        encoder_time_base.numerator(),
        encoder_time_base.denominator(),
        frame_rate.numerator(),
        frame_rate.denominator()
    );

    let mut first_n_debug = 32;

    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet)?;

            let mut decoded_frame = Video::empty();
            while decoder.receive_frame(&mut decoded_frame).is_ok() {
                let ts_opt = decoded_frame.timestamp();
                let frame_ms_unaligned: f64 = if let Some(ts) = ts_opt {
                    ts.rescale(input_time_base, ffmpeg::Rational(1, 1000)) as f64
                } else {
                    fallback_ms_from_frame_index(frame_number)
                };

                // Collect alignment samples
                alignment.add_sample(frame_ms_unaligned, path_start_ms);
                let align_offset = alignment.get_offset(path_start_ms);
                let aligned_ms = frame_ms_unaligned + align_offset;

                if first_n_debug > 0 {
                    trace!(
                        "Frame {}: pts={} frame_ms={:.3} aligned_ms={:.3}",
                        frame_number,
                        ts_opt
                            .map(|t| t.to_string())
                            .unwrap_or_else(|| "None".into()),
                        frame_ms_unaligned,
                        aligned_ms
                    );
                    first_n_debug -= 1;
                }

                let mut rgb_frame = Video::empty();
                input_to_rgb_scaler.run(&decoded_frame, &mut rgb_frame)?;
                rgb_frame.set_pts(Some(cur_pts));

                let mut clamped = false;
                if let Some(pos) =
                    find_position_for_timestamp_hermite(smoothed_path, aligned_ms, &mut clamped)
                {
                    overlay_image_on_rgb_frame(
                        &mut rgb_frame,
                        &overlay_img,
                        pos.0 as i32,
                        pos.1 as i32,
                    );
                    if frame_number % 60 == 0 {
                        trace!(
                            "Overlay frame {}: ({:.1},{:.1}) t_ms={:.3} clamped={}",
                            frame_number,
                            pos.0,
                            pos.1,
                            aligned_ms,
                            clamped
                        );
                    }
                }
                if clamped && frame_number % 60 == 0 {
                    debug!("Timestamp clamped at frame {}", frame_number);
                }

                let mut output_frame = Video::empty();
                rgb_to_output_scaler.run(&rgb_frame, &mut output_frame)?;
                output_frame.set_pts(Some(cur_pts));
                encoder.send_frame(&output_frame)?;
                let mut encoded_packet = ffmpeg::Packet::empty();
                while encoder.receive_packet(&mut encoded_packet).is_ok() {
                    encoded_packet.set_stream(ostream_index);
                    let ost_tb = octx
                        .stream(ostream_index)
                        .ok_or("Output stream not found")?
                        .time_base();
                    if ost_tb != encoder_time_base {
                        encoded_packet.rescale_ts(encoder_time_base, ost_tb);
                    }
                    encoded_packet.write_interleaved(&mut octx)?;
                }

                cur_pts += frame_duration_pts;
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
        let ts_opt = decoded_frame.timestamp();
        let frame_ms_unaligned: f64 = if let Some(ts) = ts_opt {
            ts.rescale(input_time_base, ffmpeg::Rational(1, 1000)) as f64
        } else {
            fallback_ms_from_frame_index(frame_number)
        };

        let align_offset = alignment.get_offset(path_start_ms);
        let aligned_ms = frame_ms_unaligned + align_offset;

        let mut rgb_frame = Video::empty();
        input_to_rgb_scaler.run(&decoded_frame, &mut rgb_frame)?;
        rgb_frame.set_pts(Some(cur_pts));

        let mut clamped = false;
        if let Some(pos) =
            find_position_for_timestamp_hermite(smoothed_path, aligned_ms, &mut clamped)
        {
            overlay_image_on_rgb_frame(&mut rgb_frame, &overlay_img, pos.0 as i32, pos.1 as i32);
        }
        if clamped {
            debug!(
                "Timestamp clamped during decoder flush at frame {}",
                frame_number
            );
        }

        let mut output_frame = Video::empty();
        rgb_to_output_scaler.run(&rgb_frame, &mut output_frame)?;
        output_frame.set_pts(Some(cur_pts));

        encoder.send_frame(&output_frame)?;

        let mut encoded_packet = ffmpeg::Packet::empty();
        while encoder.receive_packet(&mut encoded_packet).is_ok() {
            encoded_packet.set_stream(ostream_index);
            let ost_tb = octx
                .stream(ostream_index)
                .ok_or("Output stream not found")?
                .time_base();
            if ost_tb != encoder_time_base {
                encoded_packet.rescale_ts(encoder_time_base, ost_tb);
            }
            encoded_packet.write_interleaved(&mut octx)?;
        }

        cur_pts += frame_duration_pts;
        frame_number += 1;
    }

    // Flush encoder
    encoder.send_eof()?;
    let mut encoded_packet = ffmpeg::Packet::empty();
    while encoder.receive_packet(&mut encoded_packet).is_ok() {
        encoded_packet.set_stream(ostream_index);
        let ost_tb = octx
            .stream(ostream_index)
            .ok_or("Output stream not found")?
            .time_base();
        if ost_tb != encoder_time_base {
            encoded_packet.rescale_ts(encoder_time_base, ost_tb);
        }
        encoded_packet.write_interleaved(&mut octx)?;
    }

    octx.write_trailer()?;
    info!(
        "Render complete: frames={} duration~{:.3}s",
        frame_number,
        (cur_pts as f64)
            * (encoder_time_base.numerator() as f64 / encoder_time_base.denominator() as f64)
    );
    Ok(())
}

// ============================================================================
// Hermite Interpolation for Smooth Position Lookup
// ============================================================================

/// Finds cursor position at a given timestamp using Hermite spline interpolation.
/// Uses velocity data from PathPoints for C1 continuity at segment boundaries.
///
/// Returns (x, y) position or None if path is empty.
pub(crate) fn find_position_for_timestamp_hermite(
    path: &[PathPoint],
    timestamp_ms: f64,
    clamped: &mut bool,
) -> Option<(f64, f64)> {
    *clamped = false;
    if path.is_empty() {
        return None;
    }
    if path.len() == 1 {
        *clamped = true;
        return Some((path[0].x, path[0].y));
    }

    // Find the segment containing this timestamp
    if let Some(index) = path
        .windows(2)
        .position(|w| timestamp_ms >= w[0].timestamp_ms && timestamp_ms <= w[1].timestamp_ms)
    {
        let p1 = &path[index];
        let p2 = &path[index + 1];
        let duration_ms = p2.timestamp_ms - p1.timestamp_ms;

        if duration_ms <= 0.0 {
            return Some((p1.x, p1.y));
        }

        // Normalized time parameter t in [0, 1]
        let t = (timestamp_ms - p1.timestamp_ms) / duration_ms;

        // Convert velocities from px/s to px/segment_duration
        // v_scaled = v * duration_s = v * duration_ms / 1000
        let duration_s = duration_ms / 1000.0;
        let v1x = p1.vx * duration_s;
        let v1y = p1.vy * duration_s;
        let v2x = p2.vx * duration_s;
        let v2y = p2.vy * duration_s;

        // Hermite basis functions
        let t2 = t * t;
        let t3 = t2 * t;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0; // Position at p1
        let h10 = t3 - 2.0 * t2 + t; // Tangent at p1
        let h01 = -2.0 * t3 + 3.0 * t2; // Position at p2
        let h11 = t3 - t2; // Tangent at p2

        // Hermite interpolation
        let x = h00 * p1.x + h10 * v1x + h01 * p2.x + h11 * v2x;
        let y = h00 * p1.y + h10 * v1y + h01 * p2.y + h11 * v2y;

        return Some((x, y));
    }

    // Timestamp outside path range - clamp to endpoints
    *clamped = true;
    if timestamp_ms < path[0].timestamp_ms {
        Some((path[0].x, path[0].y))
    } else {
        let last = path.last().unwrap();
        Some((last.x, last.y))
    }
}

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

                if a > 0 {
                    let frame_idx = (y_frame as usize * stride) + (x_frame as usize * 3);
                    if frame_idx + 2 < frame_data.len() {
                        if a == 255 {
                            frame_data[frame_idx] = r;
                            frame_data[frame_idx + 1] = g;
                            frame_data[frame_idx + 2] = b;
                        } else {
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

// ============================================================================
// Catmull-Rom Spline Implementation
// ============================================================================

pub(crate) fn catmull_rom_spline(
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

pub(crate) fn interpolate_points(
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

pub(crate) fn calculate_t_j(t_i: f32, p_i: &CPoint, p_j: &CPoint, alpha: f32) -> f32 {
    let dx = p_j.x - p_i.x;
    let dy = p_j.y - p_i.y;
    t_i + (dx.powi(2) + dy.powi(2)).sqrt().powf(alpha)
}

pub(crate) fn linspace(start: f32, end: f32, num_points: usize) -> Vec<f32> {
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
// Physics-based spring follower (mass-spring-damper) with velocity output
// ============================================================================

/// Applies spring physics to smooth cursor path, outputting both position and velocity.
/// Velocity is used for Hermite interpolation to ensure C1 continuity.
pub(crate) fn spring_follow_path(points: &[CPoint], config: &VideoProcessingConfig) -> Vec<PathPoint> {
    if points.is_empty() {
        return Vec::new();
    }

    let fps = if config.frame_rate > 0 {
        config.frame_rate as f64
    } else {
        60.0
    };
    let max_dt = 2.0 / fps;

    let mut out: Vec<PathPoint> = Vec::with_capacity(points.len());

    // Derive spring parameters from perceptual inputs
    let (k, c, m) = derive_spring_params(config.responsiveness, config.smoothness);

    let omega_n = (k / m).sqrt();

    // State initialization at first target point
    let mut pos_x = points[0].x as f64;
    let mut pos_y = points[0].y as f64;
    let mut vel_x = 0.0f64;
    let mut vel_y = 0.0f64;
    let mut prev_t = points[0].timestamp_ms;

    out.push(PathPoint::with_velocity(pos_x, pos_y, prev_t, vel_x, vel_y));

    for i in 1..points.len() {
        let target = &points[i];
        let dt = ((target.timestamp_ms - prev_t) / 1000.0).max(0.0);

        // Adaptive substeps for stability
        let base_sub = if dt > max_dt {
            (dt / max_dt).ceil() as i32
        } else {
            1
        };
        let freq_sub = (dt * omega_n * 8.0).ceil() as i32;
        let substeps = base_sub.max(freq_sub.max(1));
        let h = dt / substeps as f64;

        for _ in 0..substeps {
            let dx = target.x as f64 - pos_x;
            let dy = target.y as f64 - pos_y;

            // Force = k*x - c*v (towards target)
            let ax = (k * dx - c * vel_x) / m;
            let ay = (k * dy - c * vel_y) / m;

            // Semi-implicit (symplectic) Euler
            vel_x += ax * h;
            vel_y += ay * h;
            pos_x += vel_x * h;
            pos_y += vel_y * h;
        }

        prev_t = target.timestamp_ms;
        out.push(PathPoint::with_velocity(
            pos_x,
            pos_y,
            target.timestamp_ms,
            vel_x,
            vel_y,
        ));
    }

    out
}

// ============================================================================
// Arc-length reparameterization and trapezoidal speed profile targets
// ============================================================================

pub(crate) fn cumulative_lengths(points: &[CPoint]) -> Vec<f64> {
    let mut cum: Vec<f64> = Vec::with_capacity(points.len());
    let mut s = 0.0f64;
    for (i, p) in points.iter().enumerate() {
        if i == 0 {
            cum.push(0.0);
        } else {
            let prev = &points[i - 1];
            let dx = (p.x as f64) - (prev.x as f64);
            let dy = (p.y as f64) - (prev.y as f64);
            s += (dx * dx + dy * dy).sqrt();
            cum.push(s);
        }
    }
    cum
}

pub(crate) fn position_at_distance(points: &[CPoint], cum: &[f64], s_query: f64) -> (f32, f32) {
    if points.is_empty() {
        return (0.0, 0.0);
    }
    if points.len() == 1 {
        return (points[0].x, points[0].y);
    }
    let s_query = s_query.max(0.0).min(*cum.last().unwrap_or(&0.0));
    // binary search
    let mut lo = 0usize;
    let mut hi = cum.len() - 1;
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if cum[mid] <= s_query {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let s0 = cum[lo];
    let s1 = cum[hi];
    let p0 = &points[lo];
    let p1 = &points[hi];
    if (s1 - s0).abs() < f64::EPSILON {
        return (p1.x, p1.y);
    }
    let t = ((s_query - s0) / (s1 - s0)) as f32;
    let x = p0.x + t * (p1.x - p0.x);
    let y = p0.y + t * (p1.y - p0.y);
    (x, y)
}

/// Generate targets so that each raw cursor point (click waypoint) is reached at its original timestamp.
/// Maps raw points onto the spline's arc-length, then builds trapezoidal/triangular speed profiles.
pub(crate) fn generate_targets_with_click_constraints(
    raw_points: &[CPoint],
    spline_points: &[CPoint],
    _base_ts_ms: f64,
    frame_rate: i32,
) -> Vec<CPoint> {
    if raw_points.len() < 2 || spline_points.len() < 2 {
        return spline_points.to_vec();
    }

    let fps = if frame_rate > 0 {
        frame_rate as f64
    } else {
        60.0
    };
    let dt_frame = 1.0 / fps;

    let cum = cumulative_lengths(spline_points);
    let total_s = *cum.last().unwrap_or(&0.0);
    if total_s <= 0.0 {
        return spline_points.to_vec();
    }

    // Map raw points onto arc-length by nearest point index
    let mut anchors_s: Vec<f64> = Vec::with_capacity(raw_points.len());
    let mut anchors_t: Vec<f64> = Vec::with_capacity(raw_points.len());
    for rp in raw_points {
        let mut best_i = 0usize;
        let mut best_d2 = f64::MAX;
        for (i, sp) in spline_points.iter().enumerate() {
            let dx = (sp.x as f64) - (rp.x as f64);
            let dy = (sp.y as f64) - (rp.y as f64);
            let d2 = dx * dx + dy * dy;
            if d2 < best_d2 {
                best_d2 = d2;
                best_i = i;
            }
        }
        anchors_s.push(cum[best_i]);
        anchors_t.push(rp.timestamp_ms);
    }

    // Ensure monotonic anchors along s
    for i in 1..anchors_s.len() {
        if anchors_s[i] < anchors_s[i - 1] {
            anchors_s[i] = anchors_s[i - 1];
        }
    }

    // Use documented constants for kinematics
    let amax = ACCELERATION_MAX_PX_PER_SEC2;
    let vmax = VELOCITY_MAX_PX_PER_SEC;

    let mut out: Vec<CPoint> = Vec::new();

    for seg in 0..(anchors_s.len() - 1) {
        let s0 = anchors_s[seg];
        let s1 = anchors_s[seg + 1];
        let t0 = anchors_t[seg];
        let t1 = anchors_t[seg + 1];
        let ds = (s1 - s0).max(0.0);
        let dt = ((t1 - t0) / 1000.0).max(1e-6);

        // Solve for peak velocity
        let mut v_peak = {
            let a = amax;
            let disc = (a * a) * (dt * dt) - 4.0 * a * ds;
            if disc >= 0.0 {
                let v = 0.5 * (a * dt - disc.sqrt());
                v.max(0.0)
            } else {
                amax * dt * 0.5
            }
        };
        v_peak = v_peak.min(vmax);

        let t_acc = v_peak / amax;
        let t_dec = t_acc;
        let t_cruise = (dt - t_acc - t_dec).max(0.0);
        let s_acc = 0.5 * amax * t_acc * t_acc;
        let s_cruise = v_peak * t_cruise;
        let s_dec_total = s_acc;
        let s_total = s_acc + s_cruise + s_dec_total;

        let scale = if s_total > 1e-9 {
            (ds / s_total).max(0.0)
        } else {
            1.0
        };
        let a_eff = amax * scale;
        let v_eff = v_peak * scale;
        let t_acc_eff = if a_eff > 1e-9 { v_eff / a_eff } else { 0.0 };
        let t_cruise_eff = (dt - 2.0 * t_acc_eff).max(0.0);
        let s_acc_eff = 0.5 * a_eff * t_acc_eff * t_acc_eff;

        let mut t = 0.0f64;
        while t < dt {
            let s_rel = if t <= t_acc_eff {
                0.5 * a_eff * t * t
            } else if t <= t_acc_eff + t_cruise_eff {
                let tc = t - t_acc_eff;
                s_acc_eff + v_eff * tc
            } else {
                let td = t - (t_acc_eff + t_cruise_eff);
                let s_before = s_acc_eff + v_eff * t_cruise_eff;
                let s_dec = v_eff * td - 0.5 * a_eff * td * td;
                s_before + s_dec
            };
            let s_abs = s0 + s_rel.min(ds);
            let (x, y) = position_at_distance(spline_points, &cum, s_abs);
            out.push(CPoint {
                x,
                y,
                timestamp_ms: t0 + t * 1000.0,
            });
            t += dt_frame;
        }
        // Ensure exact endpoint
        let (xe, ye) = position_at_distance(spline_points, &cum, s1);
        out.push(CPoint {
            x: xe,
            y: ye,
            timestamp_ms: t1,
        });
    }

    out
}

// ============================================================================
// Legacy FFI Functions (deprecated, use process_video_with_cursor instead)
// ============================================================================

#[cfg(feature = "legacy-api")]
#[deprecated(
    since = "0.2.0",
    note = "Use process_video_with_cursor instead. This function will be removed in a future version."
)]
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

#[cfg(feature = "legacy-api")]
#[no_mangle]
pub unsafe extern "C" fn free_smoothed_path(path: CSmoothedPath) {
    if !path.points.is_null() && path.len > 0 {
        let _ = Vec::from_raw_parts(path.points, path.len, path.len);
    }
}
