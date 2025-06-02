use std::f32;

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

    for t in t_values {
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

pub fn catmull_rom_chain(
    points: &[CPoint],
    num_points_per_segment: usize,
    alpha: f32,
    quadruple_size: usize,
) -> Vec<CPoint> {
    // Need at least quadruple_size points to form one segment.
    if points.len() < quadruple_size {
        return Vec::new();
    }

    // Pre-allocate for efficiency, if possible.
    let num_segments = points.len() - quadruple_size + 1;
    let total_capacity = num_segments * num_points_per_segment;
    let mut all_spline_points = Vec::with_capacity(total_capacity);

    // Use the `windows` iterator to get sliding windows of 4 points.
    // Each `window` is a slice `&[CPoint]` of length quadruple_size
    for window in points.windows(quadruple_size) {
        // Since CPoint is Copy, these are direct copies of the point data.
        let p0 = window[0];
        let p1 = window[1];
        let p2 = window[2];
        let p3 = window[3];

        let segment_points = catmull_rom_spline(p0, p1, p2, p3, num_points_per_segment, alpha);

        // Extend the main list with the points from the current segment.
        all_spline_points.extend(segment_points);
    }

    all_spline_points
}

// Linear interpolation between two points
fn interpolate_points(
    t_end: f32,
    t_start: f32,
    t: f32,
    p_start: &CPoint,
    p_end: &CPoint,
) -> CPoint {
    let (weight1, weight2) = if (t_end - t_start).abs() < f32::EPSILON {
        // Avoid division by zero if t_start and t_end are the same.
        // If t is also t_start, result is p_start. If t is t_end, result is p_end.
        // Default to p_start if interval is zero.
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
        // Correctly interpolate timestamp_ms (f64) using f32 weights cast to f64
        timestamp_ms: (weight1 as f64) * p_start.timestamp_ms
            + (weight2 as f64) * p_end.timestamp_ms,
    }
}

fn linspace(start: f32, end: f32, num_points: usize) -> Vec<f32> {
    if num_points == 0 {
        return Vec::new();
    }
    if num_points == 1 {
        return vec![start];
    }

    let mut result = Vec::with_capacity(num_points);
    // Calculate the step size.
    // To include both `start` and `end`, there are `num_points - 1` intervals.
    let step = (end - start) / (num_points - 1) as f32;

    for i in 0..num_points {
        let value = start + (i as f32) * step;
        result.push(value);
    }
    result
}

fn calculate_t_j(t_i: f32, p_i: &CPoint, p_j: &CPoint, alpha: f32) -> f32 {
    let x_i = p_i.x;
    let y_i = p_i.y;

    let x_j = p_j.x;
    let y_j = p_j.y;

    let dx = x_j - x_i;
    let dy = y_j - y_i;

    let l = (dx.powi(2) + dy.powi(2)).sqrt();
    t_i + l.powf(alpha)
}

#[no_mangle]
pub extern "C" fn smooth_cursor_path(
    raw_points_ptr: *const CPoint,
    raw_points_len: usize,
    points_per_segment_ptr: *const i64, // Array of point counts for each segment
    points_per_segment_len: usize,      // Length of points_per_segment_ptr
    alpha: f32,                         // Catmull-Rom alpha
    _tension: f32,  // Currently unused by Catmull-Rom, for potential physics layer
    _friction: f32, // Currently unused
    _mass: f32,     // Currently unused
) -> CSmoothedPath {
    // Basic safety checks for pointers
    if raw_points_ptr.is_null() || points_per_segment_ptr.is_null() {
        return CSmoothedPath {
            points: std::ptr::null_mut(),
            len: 0,
        };
    }

    // Create slices from raw parts (unsafe operation)
    let points_slice: &[CPoint] =
        unsafe { std::slice::from_raw_parts(raw_points_ptr, raw_points_len) };

    let frame_amount_slice: &[i64] =
        unsafe { std::slice::from_raw_parts(points_per_segment_ptr, points_per_segment_len) };

    // Check for empty inputs
    if points_slice.is_empty() || frame_amount_slice.is_empty() {
        return CSmoothedPath {
            points: std::ptr::null_mut(),
            len: 0,
        };
    }

    let quadruple_size: usize = 4; // Number of points needed to define one Catmull-Rom segment

    // Calculate the number of segments we can form
    // If points_slice.len() < quadruple_size, num_segments will be 0.
    let num_segments = points_slice.len().saturating_sub(quadruple_size - 1);

    if num_segments == 0 {
        // Not enough points to form any segment
        return CSmoothedPath {
            points: std::ptr::null_mut(),
            len: 0,
        };
    }

    // The length of frame_amount_slice must match the number of segments we can process.
    if frame_amount_slice.len() != num_segments {
        // Data mismatch: Go provided an incorrect number of segment point counts.
        // Consider logging an error here if possible, or handle as per API contract.
        return CSmoothedPath {
            points: std::ptr::null_mut(),
            len: 0,
        };
    }

    let mut all_spline_points: Vec<CPoint> = Vec::new();
    // Estimate capacity: sum of all points in frame_amount_slice
    let total_expected_points: usize = frame_amount_slice.iter().map(|&x| x as usize).sum();
    all_spline_points.reserve(total_expected_points);

    // Iterate through the raw points in windows of 'quadruple_size'
    // Each window provides P0, P1, P2, P3 for one spline segment.
    // The segment is primarily between P1 and P2.
    for (i, window) in points_slice.windows(quadruple_size).enumerate() {
        let p0 = window[0];
        let p1 = window[1];
        let p2 = window[2];
        let p3 = window[3];

        // Get the number of points to interpolate for this specific segment
        let num_points_for_this_segment = frame_amount_slice[i] as usize;

        // Ensure num_points_for_this_segment is reasonable (e.g., at least 2 if not 0)
        // If 0 or 1, catmull_rom_spline might behave unexpectedly or inefficiently depending on linspace.
        // catmull_rom_spline's linspace handles num_points = 0 or 1, returning empty or single point vec.
        if num_points_for_this_segment > 0 {
            let segment_points =
                catmull_rom_spline(p0, p1, p2, p3, num_points_for_this_segment, alpha);
            all_spline_points.extend(segment_points);
        } else {
            // If num_points_for_this_segment is 0, we add p1 to ensure connectivity
            // but typically Go side would ensure num_points_for_this_segment >= 1 or >= 2.
        }
    }

    // Prepare the result to be returned via FFI
    all_spline_points.shrink_to_fit();
    let len = all_spline_points.len();
    let ptr = all_spline_points.as_mut_ptr();
    std::mem::forget(all_spline_points); // Prevent Rust from dropping the data

    CSmoothedPath { points: ptr, len }
}

#[no_mangle]
pub extern "C" fn free_smoothed_path(path: CSmoothedPath) {
    // This function is crucial for Go to call to free the memory
    // allocated by Rust and passed back in CSmoothedPath.
    unsafe {
        if !path.points.is_null() && path.len > 0 {
            // Reconstruct the Vec from the raw parts and let it drop,
            // which deallocates the memory.
            let _ = Vec::from_raw_parts(path.points, path.len, path.len);
        }
    }
}
