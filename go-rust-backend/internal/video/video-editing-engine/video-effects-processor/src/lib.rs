// lib.rs - Foreign Function Interface boundary
mod renderer;
mod smoothing;
mod utils;
mod video;

use std::ffi::{c_char, c_void, CStr};
use std::panic::AssertUnwindSafe;
use std::slice;

pub use smoothing::CPoint; // Re-export for consistency

// ============================================================================
// FFI Type Definitions
// ============================================================================

#[repr(C)]
pub struct CSmoothedPath {
    pub points: *mut CPoint,
    pub len: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VideoProcessingConfig {
    pub smoothing_alpha: f32,
    pub responsiveness: f32,
    pub smoothness: f32,
    pub frame_rate: i32,
    pub log_level: i32,
}

type ProgressCallback = extern "C" fn(*mut c_void, f32);

// ============================================================================
// Error Codes
// ============================================================================

const SUCCESS: i32 = 0;
const ERR_NULL_POINTER: i32 = -1;
const ERR_INVALID_UTF8: i32 = -2;
#[allow(dead_code)]
const ERR_SMOOTHING_FAILED: i32 = -3;
const ERR_RENDERING_FAILED: i32 = -4;

// ============================================================================
// Main FFI Entry Point
// ============================================================================

#[no_mangle]
pub unsafe extern "C" fn process_video_with_cursor(
    input_video_path: *const c_char,
    output_video_path: *const c_char,
    cursor_sprite_path: *const c_char,
    raw_cursor_points: *const CPoint,
    raw_cursor_points_len: usize,
    config: *const VideoProcessingConfig,
    progress_callback: Option<ProgressCallback>,
    user_data: *mut c_void,
) -> i32 {
    // 1. SAFETY: Wrap the entire execution in catch_unwind
    // We use AssertUnwindSafe because we are passing raw C pointers into the closure.
    // We guarantee that if this panics, we aren't leaving external C state corrupted
    // (since we only read these pointers).
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        // 2. Null Pointer Checks (Fast Fail)
        if input_video_path.is_null()
            || output_video_path.is_null()
            || cursor_sprite_path.is_null()
            || raw_cursor_points.is_null()
            || config.is_null()
        {
            return ERR_NULL_POINTER;
        }

        // 3. String Conversions
        // Note: These borrows are valid only within this block
        let input_path = match CStr::from_ptr(input_video_path).to_str() {
            Ok(s) => s,
            Err(_) => return ERR_INVALID_UTF8,
        };
        let output_path = match CStr::from_ptr(output_video_path).to_str() {
            Ok(s) => s,
            Err(_) => return ERR_INVALID_UTF8,
        };
        let cursor_path = match CStr::from_ptr(cursor_sprite_path).to_str() {
            Ok(s) => s,
            Err(_) => return ERR_INVALID_UTF8,
        };

        // 4. Dereference Config & Slice
        let cfg = &*config;
        utils::init_logging(cfg.log_level);

        // Create slice from raw parts
        let raw_points = slice::from_raw_parts(raw_cursor_points, raw_cursor_points_len);

        // 5. Setup Progress Callback
        let progress_reporter = ProgressReporter {
            callback: progress_callback,
            user_data, // raw pointer, captured by AssertUnwindSafe
        };

        // 6. Run Internal Logic
        match process_video_internal(
            input_path,
            output_path,
            cursor_path,
            raw_points,
            cfg,
            progress_reporter,
        ) {
            Ok(_) => SUCCESS,
            Err(e) => {
                log::error!("Video processing failed: {}", e);
                ERR_RENDERING_FAILED
            }
        }
    }));

    // 7. Handle Result
    match result {
        Ok(return_code) => return_code,
        Err(e) => {
            // Log panic details if possible
            if let Some(s) = e.downcast_ref::<&str>() {
                log::error!("CRITICAL RUST PANIC: {}", s);
            } else if let Some(s) = e.downcast_ref::<String>() {
                log::error!("CRITICAL RUST PANIC: {}", s);
            } else {
                log::error!("CRITICAL RUST PANIC: Unknown cause");
            }
            // Ensure we return a strict error code so Go knows to abort cleanly
            ERR_RENDERING_FAILED
        }
    }
}

// ============================================================================
// Standalone Smoothing Function (For Testing/Preview)
// ============================================================================

#[no_mangle]
pub unsafe extern "C" fn smooth_cursor_path(
    raw_points_ptr: *const CPoint,
    raw_points_len: usize,
    _points_per_segment_ptr: *const i64,
    _points_per_segment_len: usize,
    alpha: f32,
    tension: f32,
    friction: f32,
    _mass: f32,
) -> CSmoothedPath {
    if raw_points_ptr.is_null() || raw_points_len == 0 {
        return CSmoothedPath {
            points: std::ptr::null_mut(),
            len: 0,
        };
    }

    let raw_points = slice::from_raw_parts(raw_points_ptr, raw_points_len);

    // Map legacy parameters to new API
    let responsiveness = (tension - 50.0) / 450.0;
    let smoothness = (friction - 5.0) / 45.0;

    let result = smoothing::smooth_cursor_path_dual_pass(
        raw_points,
        60, // Default frame rate for standalone
        responsiveness.clamp(0.0, 1.0),
        smoothness.clamp(0.0, 1.0),
        alpha,
    );

    // Transfer ownership to C
    let mut boxed_slice = result.into_boxed_slice();
    let len = boxed_slice.len();
    let ptr = boxed_slice.as_mut_ptr();
    std::mem::forget(boxed_slice);

    CSmoothedPath { points: ptr, len }
}

#[no_mangle]
pub unsafe extern "C" fn free_smoothed_path(path: CSmoothedPath) {
    if !path.points.is_null() {
        let _ = Vec::from_raw_parts(path.points, path.len, path.len);
    }
}

// ============================================================================
// Internal Safe Processing Function
// ============================================================================

struct ProgressReporter {
    callback: Option<ProgressCallback>,
    user_data: *mut c_void,
}

impl ProgressReporter {
    fn report(&self, percent: f32) {
        if let Some(cb) = self.callback {
            cb(self.user_data, percent);
        }
    }
}

// Unsafe Send for raw pointers (we guarantee Go handles thread safety)
unsafe impl Send for ProgressReporter {}

fn process_video_internal(
    input_path: &str,
    output_path: &str,
    cursor_path: &str,
    raw_points: &[CPoint],
    config: &VideoProcessingConfig,
    progress: ProgressReporter,
) -> Result<(), Box<dyn std::error::Error>> {
    progress.report(0.05);
    log::info!(
        "Starting processing with {} raw cursor points",
        raw_points.len()
    );

    // Step 1: Smooth cursor path
    let smoothed_points = smoothing::smooth_cursor_path_dual_pass(
        raw_points,
        config.frame_rate,
        config.responsiveness,
        config.smoothness,
        config.smoothing_alpha,
    );

    log::info!(
        "Smoothing complete. Generated {} interpolated points",
        smoothed_points.len()
    );

    if smoothed_points.is_empty() {
        log::error!(
            "Smoothing failed! Raw points: {}, Config: {:?}",
            raw_points.len(),
            config
        );
        return Err("Cursor smoothing produced no points".into());
    }

    progress.report(0.10);

    // Step 2: Load cursor sprite
    let cursor_sprite = renderer::load_cursor_sprite(cursor_path)?;
    progress.report(0.15);

    // Step 3: Process video
    video::process_video(
        input_path,
        output_path,
        &smoothed_points,
        &cursor_sprite,
        config,
        |p| progress.report(0.15 + p * 0.85),
    )?;

    progress.report(1.0);
    Ok(())
}
