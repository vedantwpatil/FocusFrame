// Dual-pass cursor path smoothing: Physics filtering + Catmull-Rom interpolation
use std::cmp::Ordering;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CPoint {
    pub x: f32,
    pub y: f32,
    pub timestamp_ms: f64,
}

// ============================================================================
// PASS 1: Physics-Based Filtering (Remove Jitter)
// ============================================================================

/// Apply spring-damper physics to filter jitter at native sample rate
pub fn apply_physics_filter(
    raw_points: &[CPoint],
    responsiveness: f32, // 0.0-1.0
    smoothness: f32,     // 0.0-1.0
) -> Vec<CPoint> {
    if raw_points.len() < 2 {
        return raw_points.to_vec();
    }

    // Map user-friendly parameters to physics constants
    let tension = 50.0 + responsiveness * 450.0; // Spring stiffness: 50-500 N/m
    let friction = 5.0 + smoothness * 45.0; // Damping: 5-50 Ns/m
    let mass = 1.0; // Inertia: 1 kg

    let mut filtered = Vec::with_capacity(raw_points.len());

    // Initialize state
    let mut x = raw_points[0].x;
    let mut y = raw_points[0].y;
    let mut vx = 0.0_f32;
    let mut vy = 0.0_f32;

    filtered.push(raw_points[0]);

    // Simulate using actual timestamp deltas
    for i in 1..raw_points.len() {
        let dt = ((raw_points[i].timestamp_ms - raw_points[i - 1].timestamp_ms) / 1000.0) as f32;
        let dt = dt.clamp(0.001, 0.1); // Prevent instability from timestamp glitches

        let target_x = raw_points[i].x;
        let target_y = raw_points[i].y;

        // Spring-damper force calculation
        let dx = target_x - x;
        let dy = target_y - y;
        let fx = tension * dx - friction * vx;
        let fy = tension * dy - friction * vy;

        // Semi-implicit Euler integration (stable)
        let ax = fx / mass;
        let ay = fy / mass;
        vx += ax * dt;
        vy += ay * dt;
        x += vx * dt;
        y += vy * dt;

        filtered.push(CPoint {
            x,
            y,
            timestamp_ms: raw_points[i].timestamp_ms,
        });
    }

    filtered
}

// ============================================================================
// PASS 2: Catmull-Rom Spline Interpolation (Upsample to Frame Rate)
// ============================================================================

/// Interpolate sparse points to match video frame rate using Catmull-Rom splines
pub fn interpolate_to_framerate(
    clean_points: &[CPoint],
    frame_rate: i32,
    _alpha: f32,
) -> Vec<CPoint> {
    // 1. Handle Empty Input
    if clean_points.is_empty() {
        return Vec::new();
    }

    // 2. Handle Single Point (Static Cursor)
    if clean_points.len() == 1 {
        // Return at least one point so video processing doesn't fail
        return vec![clean_points[0]];
    }

    let start_time = clean_points.first().unwrap().timestamp_ms;
    let end_time = clean_points.last().unwrap().timestamp_ms;

    // 3. Handle Zero Duration (Start == End)
    if (end_time - start_time).abs() < 1e-6 {
        return vec![clean_points[0]];
    }

    // Calculate exact number of frames
    // Use max(1) to ensure we always produce points if duration > 0
    let num_frames = ((end_time - start_time) / 1000.0 * frame_rate as f64).ceil() as usize;
    let num_frames = num_frames.max(1);

    let mut dense_path = Vec::with_capacity(num_frames);
    let frame_dur = 1000.0 / frame_rate as f64;

    for i in 0..num_frames {
        let t_target = start_time + i as f64 * frame_dur;

        // Safety: Don't overshoot the end time significantly due to float math
        if t_target > end_time + frame_dur {
            break;
        }

        let idx = match clean_points
            .binary_search_by(|p| p.timestamp_ms.partial_cmp(&t_target).unwrap())
        {
            Ok(i) => i,
            Err(i) => i,
        };

        let len = clean_points.len();
        // Safe indexing with boundary clamping
        let i1 = idx.min(len - 1);
        let i0 = i1.saturating_sub(1);
        let i2 = (i1 + 1).min(len - 1);
        let i3 = (i1 + 2).min(len - 1);

        let p0 = &clean_points[i0];
        let p1 = &clean_points[i1];
        let p2 = &clean_points[i2];
        let p3 = &clean_points[i3];

        // If time interval is tiny, just use p1 position
        if (p2.timestamp_ms - p1.timestamp_ms).abs() < 1e-6 {
            dense_path.push(CPoint {
                x: p1.x,
                y: p1.y,
                timestamp_ms: t_target,
            });
            continue;
        }

        let t = t_target as f32;
        let x = catmull_rom_1d(
            t,
            p0.timestamp_ms as f32,
            p1.timestamp_ms as f32,
            p2.timestamp_ms as f32,
            p3.timestamp_ms as f32,
            p0.x,
            p1.x,
            p2.x,
            p3.x,
        );
        let y = catmull_rom_1d(
            t,
            p0.timestamp_ms as f32,
            p1.timestamp_ms as f32,
            p2.timestamp_ms as f32,
            p3.timestamp_ms as f32,
            p0.y,
            p1.y,
            p2.y,
            p3.y,
        );

        dense_path.push(CPoint {
            x,
            y,
            timestamp_ms: t_target,
        });
    }

    // Fallback: If logic produced 0 points (e.g. num_frames calculation oddity), force 1 point
    if dense_path.is_empty() {
        dense_path.push(clean_points[0]);
    }

    dense_path
}

/// Evaluate Catmull-Rom spline at parameter t using Barry-Goldman algorithm
#[allow(dead_code)]
fn catmull_rom_point(
    t: f32,
    p0: &CPoint,
    p1: &CPoint,
    p2: &CPoint,
    p3: &CPoint,
    alpha: f32,
) -> (f32, f32) {
    // Knot intervals based on chord length
    let d01 = distance(p0, p1).powf(alpha).max(1e-6);
    let d12 = distance(p1, p2).powf(alpha).max(1e-6);
    let d23 = distance(p2, p3).powf(alpha).max(1e-6);

    let t0 = 0.0;
    let t1 = t0 + d01;
    let t2 = t1 + d12;
    let t3 = t2 + d23;

    let t_mapped = t1 + t * d12;

    let x = catmull_rom_1d(t_mapped, t0, t1, t2, t3, p0.x, p1.x, p2.x, p3.x);
    let y = catmull_rom_1d(t_mapped, t0, t1, t2, t3, p0.y, p1.y, p2.y, p3.y);

    (x, y)
}

/// Barry-Goldman recursive formula for 1D Catmull-Rom interpolation
fn catmull_rom_1d(
    t: f32,
    t0: f32,
    t1: f32,
    t2: f32,
    t3: f32,
    p0: f32,
    p1: f32,
    p2: f32,
    p3: f32,
) -> f32 {
    // Helper closure for safe division
    let safe_lerp = |start_val: f32, end_val: f32, start_t: f32, end_t: f32, current_t: f32| {
        if (end_t - start_t).abs() < 1e-6 {
            start_val // Avoid NaN if timestamps are identical
        } else {
            let f = (current_t - start_t) / (end_t - start_t);
            start_val + (end_val - start_val) * f
        }
    };

    // Level 1 (Linear)
    let a1 = safe_lerp(p0, p1, t0, t1, t);
    let a2 = safe_lerp(p1, p2, t1, t2, t);
    let a3 = safe_lerp(p2, p3, t2, t3, t);

    // Level 2 (Quadratic)
    let b1 = safe_lerp(a1, a2, t0, t2, t);
    let b2 = safe_lerp(a2, a3, t1, t3, t);

    // Level 3 (Cubic) - The Result
    safe_lerp(b1, b2, t1, t2, t)
}

#[allow(dead_code)]
fn distance(a: &CPoint, b: &CPoint) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    (dx * dx + dy * dy).sqrt()
}

#[allow(dead_code)]
fn find_segment_index(points: &[CPoint], timestamp: f64) -> usize {
    match points.binary_search_by(|p| {
        if p.timestamp_ms < timestamp {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }) {
        Ok(idx) => idx,
        Err(idx) => idx,
    }
}

// ============================================================================
// PUBLIC API: Complete Dual-Pass Pipeline
// ============================================================================

/// Complete smoothing pipeline: Physics filtering + Spline interpolation
pub fn smooth_cursor_path_dual_pass(
    raw_points: &[CPoint],
    frame_rate: i32,
    responsiveness: f32, // 0.0-1.0 (controls physics spring stiffness)
    smoothness: f32,     // 0.0-1.0 (controls physics damping)
    spline_alpha: f32,   // 0.5 recommended (centripetal Catmull-Rom)
) -> Vec<CPoint> {
    if raw_points.is_empty() {
        return Vec::new();
    }

    // Normalize timestamps to milliseconds (detect if input is in seconds)
    let normalized_points = normalize_to_relative_ms(raw_points);

    let filtered = apply_physics_filter(&normalized_points, responsiveness, smoothness);
    let upsampled = interpolate_to_framerate(&filtered, frame_rate, spline_alpha);

    upsampled
}

/// Detect timestamp units and convert to milliseconds if needed.
/// Heuristic: If the last timestamp is < 10000 and duration < 1000, assume seconds.
fn normalize_to_relative_ms(points: &[CPoint]) -> Vec<CPoint> {
    if points.is_empty() {
        return Vec::new();
    }

    let start_offset = points[0].timestamp_ms;

    // Create relative timeline first (removes Unix Epoch noise)
    let mut relative_points: Vec<CPoint> = points
        .iter()
        .map(|p| CPoint {
            x: p.x,
            y: p.y,
            timestamp_ms: p.timestamp_ms - start_offset,
        })
        .collect();

    let duration = relative_points.last().unwrap().timestamp_ms;

    // HEURISTIC: If relative duration is small (< 1000), it's definitely Seconds.
    // (A 1000ms video is 1 second, unlikely to be the full recording).
    // Screen recordings are typically 5s - 300s.
    if duration > 0.0 && duration < 1000.0 {
        log::info!(
            "Detected SECONDS (Duration: {:.2}s). Converting to MS.",
            duration
        );
        for p in &mut relative_points {
            p.timestamp_ms *= 1000.0;
        }
    } else {
        log::info!(
            "Detected MILLISECONDS (Duration: {:.2}ms). Keeping units.",
            duration
        );
    }

    relative_points
}
