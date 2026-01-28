use crate::renderer::{composite_cursor_subpixel, CursorSprite};
use crate::smoothing::CPoint;
use crate::VideoProcessingConfig;
use ffmpeg::format::{input, output, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context as ScalerContext, flag::Flags};
use ffmpeg::util::frame::video::Video as VideoFrame;
use ffmpeg::{codec, encoder, Error as FfmpegError, Packet, Rational};
use ffmpeg_next as ffmpeg;
use std::error::Error;

// ============================================================================
// Main Video Processing Function
// ============================================================================

pub fn process_video(
    input_path: &str,
    output_path: &str,
    cursor_points: &[CPoint],
    cursor_sprite: &CursorSprite,
    config: &VideoProcessingConfig,
    mut progress_callback: impl FnMut(f32),
) -> Result<(), Box<dyn Error>> {
    log::info!(
        "Starting video processing: {} -> {}",
        input_path,
        output_path
    );

    ffmpeg::init()?;
    progress_callback(0.0);

    // 1. Open Input
    let mut input_ctx = input(&input_path)?;
    let video_stream = input_ctx
        .streams()
        .best(Type::Video)
        .ok_or("No video stream found")?;
    let video_stream_idx = video_stream.index();

    // 2. Create Decoder
    let decoder_context = codec::context::Context::from_parameters(video_stream.parameters())?;
    let mut decoder = decoder_context.decoder().video()?;

    log::info!(
        "Input found: {}x{} (Codec: {:?})",
        decoder.width(),
        decoder.height(),
        decoder.id()
    );

    // 3. Configure Output
    let mut output_ctx = output(&output_path)?;
    // We force the output frame rate from config (typically 60)
    let output_framerate = Rational::new(config.frame_rate, 1);

    // Create Encoder (H.264)
    let mut encoder = create_video_encoder(
        decoder.width(),
        decoder.height(),
        output_framerate,
        &mut output_ctx,
    )?;

    progress_callback(0.05);

    // 4. Setup Filter Graph (VFR -> CFR + Pixel Format Conversion)
    // We must manually add and link filters since parse() doesn't connect to existing contexts
    let mut filter_graph = ffmpeg::filter::Graph::new();

    // A. Source Filter ("buffer")
    // Describes the incoming frames from the decoder
    let buffer_args = format!(
        "video_size={}x{}:pix_fmt={}:time_base={}/{}:pixel_aspect={}/{}",
        decoder.width(),
        decoder.height(),
        decoder.format() as i32,
        video_stream.time_base().numerator(),
        video_stream.time_base().denominator(),
        decoder.aspect_ratio().numerator(),
        decoder.aspect_ratio().denominator(),
    );

    let mut filter_src_ctx = filter_graph.add(
        &ffmpeg::filter::find("buffer").ok_or("buffer filter not found")?,
        "src",
        &buffer_args,
    )?;

    // B. FPS Filter (VFR -> CFR conversion)
    let fps_args = format!("fps={}:round=near", config.frame_rate);
    let mut fps_filter = filter_graph.add(
        &ffmpeg::filter::find("fps").ok_or("fps filter not found")?,
        "fps",
        &fps_args,
    )?;

    // C. Format Filter (convert to RGBA for cursor overlay)
    let mut format_filter = filter_graph.add(
        &ffmpeg::filter::find("format").ok_or("format filter not found")?,
        "format",
        "pix_fmts=rgba",
    )?;

    // D. Sink Filter ("buffersink")
    let mut filter_sink_ctx = filter_graph.add(
        &ffmpeg::filter::find("buffersink").ok_or("buffersink filter not found")?,
        "sink",
        "",
    )?;

    // E. Link the filters: buffer -> fps -> format -> buffersink
    log::info!(
        "Building filter graph: buffer -> fps={} -> format=rgba -> buffersink",
        config.frame_rate
    );

    filter_src_ctx.link(0, &mut fps_filter, 0);
    fps_filter.link(0, &mut format_filter, 0);
    format_filter.link(0, &mut filter_sink_ctx, 0);

    filter_graph.validate()?;

    log::info!("Filter graph configured successfully");

    // 5. Scaler for Final Output (RGBA -> YUV420P for H.264)
    let mut reverse_scaler = ScalerContext::get(
        Pixel::RGBA,
        decoder.width(),
        decoder.height(),
        encoder.format(),
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR,
    )?;

    progress_callback(0.10);

    // 6. Pre-calculate Cursor Lookup Table
    let cursor_lookup = build_cursor_lookup(cursor_points);

    // Calculate stats for progress
    let start_ts = cursor_points.first().map(|p| p.timestamp_ms).unwrap_or(0.0);
    let end_ts = cursor_points.last().map(|p| p.timestamp_ms).unwrap_or(0.0);
    let estimated_total_frames = ((end_ts - start_ts) / 1000.0 * config.frame_rate as f64) as u64;

    log::info!(
        "Cursor duration: {:.2}s ({} frames estimated)",
        (end_ts - start_ts) / 1000.0,
        estimated_total_frames
    );

    // Write Header
    output_ctx.write_header()?;

    // 7. Processing Loop
    let mut frame_count = 0i64;

    for (stream, packet) in input_ctx.packets() {
        if stream.index() == video_stream_idx {
            decoder.send_packet(&packet)?;

            let mut raw_frame = VideoFrame::empty();
            while decoder.receive_frame(&mut raw_frame).is_ok() {
                // Push raw VFR frame into the filter graph source
                filter_src_ctx.source().add(&raw_frame)?;

                // Pull guaranteed CFR frames (60fps RGBA) from sink
                while let Ok(mut cfr_frame) = read_frame_from_sink(&mut filter_sink_ctx) {
                    if frame_count % 60 == 0 {
                        // Log once per second of video
                        let sink_view = filter_sink_ctx.sink();
                        let time_base = sink_view.time_base();
                        let pts = cfr_frame.pts().unwrap_or(0);
                        let video_t = pts as f64 * f64::from(time_base);

                        // Assuming 'cursor_lookup' is your Spline/Smoothing struct
                        // You might need to expose start_time/end_time methods on it
                        log::info!(
                            "Frame #{}: PTS={} (Time={:.4}s) | Querying Spline...",
                            frame_count,
                            pts,
                            video_t
                        );
                    }
                    process_single_frame(
                        &mut cfr_frame,
                        &mut encoder,
                        &mut reverse_scaler,
                        &mut output_ctx,
                        cursor_sprite,
                        &cursor_lookup,
                        frame_count,
                        &mut progress_callback,
                        estimated_total_frames,
                    )?;
                    frame_count += 1;
                }
            }
        }
    }

    // 8. Flush Decoder
    log::info!("Flushing decoder...");
    decoder.send_eof()?;
    let mut raw_frame = VideoFrame::empty();
    while decoder.receive_frame(&mut raw_frame).is_ok() {
        filter_src_ctx.source().add(&raw_frame)?;
        while let Ok(mut cfr_frame) = read_frame_from_sink(&mut filter_sink_ctx) {
            process_single_frame(
                &mut cfr_frame,
                &mut encoder,
                &mut reverse_scaler,
                &mut output_ctx,
                cursor_sprite,
                &cursor_lookup,
                frame_count,
                &mut progress_callback,
                estimated_total_frames,
            )?;
            frame_count += 1;
        }
    }

    // 9. Flush Filter Graph
    log::info!("Flushing filter graph...");
    filter_src_ctx.source().flush()?; // Signal EOF to filter
    while let Ok(mut cfr_frame) = read_frame_from_sink(&mut filter_sink_ctx) {
        process_single_frame(
            &mut cfr_frame,
            &mut encoder,
            &mut reverse_scaler,
            &mut output_ctx,
            cursor_sprite,
            &cursor_lookup,
            frame_count,
            &mut progress_callback,
            estimated_total_frames,
        )?;
        frame_count += 1;
    }

    // 10. Flush Encoder
    log::info!("Flushing encoder...");
    encoder.send_eof()?;
    encode_and_write(&mut encoder, &mut output_ctx)?;

    // Write Trailer
    output_ctx.write_trailer()?;

    progress_callback(1.0);
    log::info!(
        "Video processing complete. Total frames generated: {}",
        frame_count
    );

    Ok(())
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Reads a frame from the filter sink, handling the Result wrapper
fn read_frame_from_sink(
    filter_sink: &mut ffmpeg::filter::Context,
) -> Result<VideoFrame, FfmpegError> {
    let mut frame = VideoFrame::empty();
    // sink().frame() pulls a filtered frame from the graph
    filter_sink.sink().frame(&mut frame)?;
    Ok(frame)
}

fn process_single_frame(
    cfr_frame: &mut VideoFrame,
    encoder: &mut encoder::Video,
    reverse_scaler: &mut ScalerContext,
    output_ctx: &mut ffmpeg::format::context::Output,
    cursor_sprite: &CursorSprite,
    cursor_lookup: &[(f64, f32, f32)],
    frame_count: i64,
    progress_callback: &mut impl FnMut(f32),
    total_estimated: u64,
) -> Result<(), Box<dyn Error>> {
    // A. Calculate Timing based on Frame Count
    // Since we forced CFR, Time = Frame / FPS
    let time_base_seconds =
        encoder.time_base().numerator() as f64 / encoder.time_base().denominator() as f64;
    let timestamp_ms = frame_count as f64 * time_base_seconds * 1000.0;

    // B. Cursor Overlay
    let (cx, cy) = interpolate_cursor_position(cursor_lookup, timestamp_ms);
    overlay_cursor_on_frame(cfr_frame, cursor_sprite, cx, cy)?;

    // C. Convert to YUV (H.264 format)
    let mut yuv_frame = VideoFrame::empty();
    reverse_scaler.run(cfr_frame, &mut yuv_frame)?;

    // D. Encode
    yuv_frame.set_pts(Some(frame_count));
    encoder.send_frame(&yuv_frame)?;
    encode_and_write(encoder, output_ctx)?;

    // E. Progress Reporting
    if frame_count % 30 == 0 && total_estimated > 0 {
        let p = (frame_count as f64 / total_estimated as f64) as f32;
        progress_callback(0.10 + p * 0.85);
    }

    Ok(())
}

fn overlay_cursor_on_frame(
    frame: &mut VideoFrame,
    cursor_sprite: &CursorSprite,
    x: f32,
    y: f32,
) -> Result<(), Box<dyn Error>> {
    // Frame is guaranteed RGBA by filter graph
    let width = frame.width();
    let height = frame.height();

    // IMPORTANT: Handle frame stride (pitch)
    // composite_cursor_subpixel must iterate rows using stride, not just width*4
    let _stride = frame.stride(0); // TODO: Pass stride to renderer for non-contiguous frames
    let data = frame.data_mut(0);

    // Call renderer (Update your renderer.rs to accept stride!)
    // If renderer.rs is not updated, this assumes stride == width * 4 (Risky but common)
    composite_cursor_subpixel(data, width, height, cursor_sprite, x, y);

    Ok(())
}

fn create_video_encoder(
    width: u32,
    height: u32,
    frame_rate: Rational,
    output_ctx: &mut ffmpeg::format::context::Output,
) -> Result<encoder::Video, Box<dyn Error>> {
    let global_header = output_ctx
        .format()
        .flags()
        .contains(ffmpeg::format::flag::Flags::GLOBAL_HEADER);
    let codec = encoder::find(codec::Id::H264).ok_or("H264 encoder not found")?;
    let mut output_stream = output_ctx.add_stream(Some(codec))?;

    let mut encoder = codec::context::Context::new_with_codec(codec)
        .encoder()
        .video()?;

    encoder.set_width(width);
    encoder.set_height(height);
    encoder.set_format(Pixel::YUV420P);
    encoder.set_frame_rate(Some(frame_rate));
    // Set timebase to 1/FPS (standard for CFR)
    encoder.set_time_base(Rational::new(
        frame_rate.denominator(),
        frame_rate.numerator(),
    ));

    if global_header {
        encoder.set_flags(codec::flag::Flags::GLOBAL_HEADER);
    }

    // Encoder Options (x264)
    let mut opts = ffmpeg::Dictionary::new();
    opts.set("preset", "fast");
    opts.set("crf", "18");

    let opened = encoder.open_with(opts)?;
    output_stream.set_parameters(&opened);

    Ok(opened)
}

fn encode_and_write(
    encoder: &mut encoder::Video,
    output_ctx: &mut ffmpeg::format::context::Output,
) -> Result<(), FfmpegError> {
    let mut packet = Packet::empty();
    while encoder.receive_packet(&mut packet).is_ok() {
        packet.set_stream(0);

        // Rescale timestamps from encoder time_base to output stream time_base
        let encoder_tb = encoder.time_base();
        let stream_tb = output_ctx.stream(0).map(|s| s.time_base()).unwrap_or(encoder_tb);
        packet.rescale_ts(encoder_tb, stream_tb);

        packet.write_interleaved(output_ctx)?;
    }
    Ok(())
}

fn build_cursor_lookup(cursor_points: &[CPoint]) -> Vec<(f64, f32, f32)> {
    if cursor_points.is_empty() {
        return Vec::new();
    }
    let start_time = cursor_points[0].timestamp_ms;
    cursor_points
        .iter()
        .map(|p| (p.timestamp_ms - start_time, p.x, p.y))
        .collect()
}

fn interpolate_cursor_position(lookup: &[(f64, f32, f32)], timestamp_ms: f64) -> (f32, f32) {
    if lookup.is_empty() {
        return (0.0, 0.0);
    }

    let idx = match lookup.binary_search_by(|p| p.0.partial_cmp(&timestamp_ms).unwrap()) {
        Ok(i) => i,
        Err(i) => i,
    };

    if idx == 0 {
        return (lookup[0].1, lookup[0].2);
    }
    if idx >= lookup.len() {
        let last = lookup.last().unwrap();
        return (last.1, last.2);
    }

    let (t0, x0, y0) = lookup[idx - 1];
    let (t1, x1, y1) = lookup[idx];

    let dt = t1 - t0;
    if dt < 1e-6 {
        return (x1, y1);
    }

    let t = ((timestamp_ms - t0) / dt) as f32;
    (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t)
}
