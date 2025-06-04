use std::f32;
use std::fs::File;
use std::io::{BufWriter, Write};

fn export_points_to_csv(filename: &str, points: &[CPoint]) -> std::io::Result<()> {
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "x,y,timestamp_ms")?;
    for p in points {
        writeln!(writer, "{},{},{}", p.x, p.y, p.timestamp_ms)?;
    }
    Ok(())
}

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
    let c_points = &[p0, p1, p2, p3];

    // Debugging
    let _ = export_points_to_csv("control_points.csv", c_points);

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
    // Debugging
    let _ = export_points_to_csv("spline_points.csv", &points);

    points
}

pub fn catmull_rom_chain(
    points: &[CPoint],
    num_points_per_segment: usize,
    alpha: f32,
    quadruple_size: usize,
) -> Vec<CPoint> {
    // Need at least quadruple_size points to form one segment
    if points.len() < quadruple_size {
        return Vec::new();
    }

    // Pre-allocate for efficiency, if possible.
    let num_segments = points.len() - quadruple_size + 1;
    let total_capacity = num_segments * num_points_per_segment;
    let mut all_spline_points = Vec::with_capacity(total_capacity);

    // Use the `windows` iterator to get sliding windows of 4 points
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
        // Avoid division by zero if t_start and t_end are the same
        // If t is also t_start, result is p_start. If t is t_end, result is p_end
        // Default to p_start if interval is zero
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
    // If points_slice.len() < quadruple_size, num_segments will be 0
    let num_segments = points_slice.len().saturating_sub(quadruple_size - 1);

    if num_segments == 0 {
        // Not enough points to form any segment
        return CSmoothedPath {
            points: std::ptr::null_mut(),
            len: 0,
        };
    }

    // The length of frame_amount_slice must match the number of segments we can process
    if frame_amount_slice.len() != num_segments {
        // Data mismatch: Go provided an incorrect number of segment point counts
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

#[cfg(test)]
mod tests {
    use super::*;
    const EPSILON_F32: f32 = 1e-6;
    const EPSILON_F64: f64 = 1e-9;

    fn cpoints_are_close(p1: &CPoint, p2: &CPoint) -> bool {
        (p1.x - p2.x).abs() < EPSILON_F32
            && (p1.y - p2.y).abs() < EPSILON_F32
            && (p1.timestamp_ms - p2.timestamp_ms).abs() < EPSILON_F64
    }

    // Helper to create CPoint easily
    fn cp(x: f32, y: f32, ts: f64) -> CPoint {
        CPoint {
            x,
            y,
            timestamp_ms: ts,
        }
    }

    #[test]
    fn test_interpolate_points() {
        let p_start = cp(0.0, 0.0, 0.0);
        let p_end = cp(10.0, 20.0, 1000.0);

        // Test interpolation at start
        let interpolated_at_start = interpolate_points(1.0, 0.0, 0.0, &p_start, &p_end);
        assert!(cpoints_are_close(&interpolated_at_start, &p_start));

        // Test interpolation at end
        let interpolated_at_end = interpolate_points(1.0, 0.0, 1.0, &p_start, &p_end);
        assert!(cpoints_are_close(&interpolated_at_end, &p_end));

        // Test interpolation at midpoint
        let mid_expected = cp(5.0, 10.0, 500.0);
        let interpolated_at_mid = interpolate_points(1.0, 0.0, 0.5, &p_start, &p_end);
        assert!(cpoints_are_close(&interpolated_at_mid, &mid_expected));

        // Test zero interval
        let interpolated_zero_interval = interpolate_points(0.0, 0.0, 0.0, &p_start, &p_end);
        assert!(cpoints_are_close(&interpolated_zero_interval, &p_start));
    }

    #[test]
    fn test_linspace() {
        // Test num_points = 0
        let values_0 = linspace(0.0, 1.0, 0);
        assert!(values_0.is_empty());

        // Test num_points = 1
        let values_1 = linspace(0.0, 1.0, 1);
        assert_eq!(values_1.len(), 1);
        assert!((values_1[0] - 0.0).abs() < EPSILON_F32);

        // Test num_points = N
        let values_n = linspace(0.0, 1.0, 3);
        assert_eq!(values_n.len(), 3);
        assert!((values_n[0] - 0.0).abs() < EPSILON_F32);
        assert!((values_n[1] - 0.5).abs() < EPSILON_F32);
        assert!((values_n[2] - 1.0).abs() < EPSILON_F32);
    }

    #[test]
    fn test_calculate_t_j() {
        let p_i = cp(0.0, 0.0, 0.0);
        let p_j = cp(3.0, 4.0, 100.0); // Distance is 5.0
        let t_i = 0.0;

        // Alpha = 0.0 (uniform)
        let t_j_uniform = calculate_t_j(t_i, &p_i, &p_j, 0.0); // 5.0^0 = 1.0
        assert!((t_j_uniform - (t_i + 1.0)).abs() < EPSILON_F32);

        // Alpha = 0.5 (centripetal)
        let t_j_centripetal = calculate_t_j(t_i, &p_i, &p_j, 0.5); // sqrt(5.0)
        assert!((t_j_centripetal - (t_i + 5.0f32.sqrt())).abs() < EPSILON_F32);

        // Alpha = 1.0 (chordal)
        let t_j_chordal = calculate_t_j(t_i, &p_i, &p_j, 1.0); // 5.0^1 = 5.0
        assert!((t_j_chordal - (t_i + 5.0)).abs() < EPSILON_F32);
    }

    #[test]
    fn test_catmull_rom_spline_interpolation_ends() {
        let p0 = cp(0.0, 0.0, 0.0);
        let p1 = cp(1.0, 1.0, 100.0);
        let p2 = cp(2.0, 0.0, 200.0);
        let p3 = cp(3.0, 1.0, 300.0);
        let alpha = 0.5; // Centripetal

        // Test with num_points = 1
        let points_1 = catmull_rom_spline(p0, p1, p2, p3, 1, alpha);
        assert_eq!(points_1.len(), 1);
        assert!(
            cpoints_are_close(&points_1[0], &p1),
            "With num_points=1, spline should return P1. Got: {:?}",
            points_1[0]
        );

        // Test with num_points >= 2
        let num_points = 400;
        let points = catmull_rom_spline(p0, p1, p2, p3, num_points, alpha);
        assert_eq!(points.len(), num_points);
        // First point of the segment must be P1
        assert!(
            cpoints_are_close(&points[0], &p1),
            "First point should be P1. Got: {:?}",
            points[0]
        );
        // Last point of the segment must be P2
        assert!(
            cpoints_are_close(&points[num_points - 1], &p2),
            "Last point should be P2. Got: {:?}",
            points[num_points - 1]
        );
    }

    #[test]
    fn test_catmull_rom_spline_collinear_points() {
        let p0 = cp(0.0, 0.0, 0.0);
        let p1 = cp(1.0, 0.0, 100.0);
        let p2 = cp(2.0, 0.0, 200.0);
        let p3 = cp(3.0, 0.0, 300.0);
        let alpha = 0.5;
        let num_points = 5;

        let points = catmull_rom_spline(p0, p1, p2, p3, num_points, alpha);
        assert_eq!(points.len(), num_points);
        for (i, point) in points.iter().enumerate() {
            assert!(
                (point.y - 0.0).abs() < EPSILON_F32,
                "Point {} y-coordinate should be 0. Got: {}",
                i,
                point.y
            );
            // X should be between P1.x and P2.x
            assert!(
                point.x >= p1.x - EPSILON_F32 && point.x <= p2.x + EPSILON_F32,
                "Point {} x-coordinate out of P1-P2 range. Got: {}",
                i,
                point.x
            );
            // Timestamp should be between P1.ts and P2.ts
            assert!(
                point.timestamp_ms >= p1.timestamp_ms - EPSILON_F64
                    && point.timestamp_ms <= p2.timestamp_ms + EPSILON_F64,
                "Point {} timestamp out of P1-P2 range. Got: {}",
                i,
                point.timestamp_ms
            );
        }
        assert!(cpoints_are_close(&points[0], &p1));
        assert!(cpoints_are_close(&points[num_points - 1], &p2));
    }

    #[test]
    fn test_catmull_rom_chain_basic() {
        let control_points = vec![
            cp(0.0, 0.0, 0.0),
            cp(1.0, 1.0, 100.0),
            cp(2.0, 0.0, 200.0),
            cp(3.0, 1.0, 300.0),
            cp(4.0, 0.0, 400.0), // P4
        ];
        let num_points_per_segment = 3; // P1, one intermediate, P2
        let alpha = 0.5;
        let quadruple_size = 4;

        // Test with insufficient points
        let short_points = &control_points[0..3];
        let chain_short =
            catmull_rom_chain(short_points, num_points_per_segment, alpha, quadruple_size);
        assert!(
            chain_short.is_empty(),
            "Chain with < 4 points should be empty."
        );

        // Test with 4 points (1 segment)
        let four_points = &control_points[0..4];
        let chain_one_segment =
            catmull_rom_chain(four_points, num_points_per_segment, alpha, quadruple_size);
        assert_eq!(
            chain_one_segment.len(),
            num_points_per_segment,
            "Chain with 4 points should yield one segment's points."
        );
        assert!(cpoints_are_close(&chain_one_segment[0], &control_points[1])); // P1
        assert!(cpoints_are_close(
            &chain_one_segment[num_points_per_segment - 1],
            &control_points[2]
        )); // P2

        // Test with 5 points (2 segments)
        // Segment 1: P0,P1,P2,P3 -> interpolates P1-P2
        // Segment 2: P1,P2,P3,P4 -> interpolates P2-P3
        let chain_two_segments = catmull_rom_chain(
            &control_points,
            num_points_per_segment,
            alpha,
            quadruple_size,
        );
        assert_eq!(
            chain_two_segments.len(),
            2 * num_points_per_segment,
            "Chain with 5 points should yield two segments' points."
        );

        // Check P1, P2 for first segment
        assert!(cpoints_are_close(
            &chain_two_segments[0],
            &control_points[1]
        ));
        assert!(cpoints_are_close(
            &chain_two_segments[num_points_per_segment - 1],
            &control_points[2]
        ));

        // Check P2, P3 for second segment
        assert!(cpoints_are_close(
            &chain_two_segments[num_points_per_segment],
            &control_points[2]
        ));
        assert!(cpoints_are_close(
            &chain_two_segments[2 * num_points_per_segment - 1],
            &control_points[3]
        ));
    }

    // Tests for `smooth_cursor_path` are more involved due to FFI and raw pointers.
    // The core logic is similar to `catmull_rom_chain` but with variable points per segment.
    // You can test its internal logic by constructing inputs carefully.
    #[test]
    fn test_smooth_cursor_path_logic() {
        let control_points_vec = [
            cp(0.0, 0.0, 0.0),   // P0
            cp(1.0, 1.0, 100.0), // P1
            cp(2.0, 0.0, 200.0), // P2
            cp(3.0, 1.0, 300.0), // P3
            cp(4.0, 0.0, 400.0), // P4
        ];
        let alpha = 0.5;

        // Scenario: 2 segments. First segment 3 points, second segment 2 points.
        // Segment 1 (P0-P1-P2-P3) -> interpolates P1-P2
        // Segment 2 (P1-P2-P3-P4) -> interpolates P2-P3
        let points_per_segment_vec: Vec<i64> = vec![3, 2]; // 3 points for P1-P2, 2 points for P2-P3

        let raw_points_ptr = control_points_vec.as_ptr();
        let raw_points_len = control_points_vec.len();
        let points_per_segment_ptr = points_per_segment_vec.as_ptr();
        let points_per_segment_len = points_per_segment_vec.len();

        // Expected number of segments: 5 (control_points) - 4 (quad_size) + 1 = 2 segments.
        // This matches points_per_segment_len.

        let smoothed_path = smooth_cursor_path(
            raw_points_ptr,
            raw_points_len,
            points_per_segment_ptr,
            points_per_segment_len,
            alpha,
            0.0,
            0.0,
            0.0, // Unused tension, friction, mass
        );

        assert_ne!(
            smoothed_path.points,
            std::ptr::null_mut(),
            "Path points should not be null"
        );
        let expected_total_points = points_per_segment_vec.iter().sum::<i64>() as usize;
        assert_eq!(
            smoothed_path.len, expected_total_points,
            "Smoothed path length mismatch"
        );

        if smoothed_path.len > 0 && !smoothed_path.points.is_null() {
            let result_slice =
                unsafe { std::slice::from_raw_parts(smoothed_path.points, smoothed_path.len) };

            // Segment 1 (P1-P2) should have 3 points
            assert!(cpoints_are_close(&result_slice[0], &control_points_vec[1])); // P1
            assert!(cpoints_are_close(&result_slice[2], &control_points_vec[2])); // P2

            // Segment 2 (P2-P3) should have 2 points
            assert!(cpoints_are_close(&result_slice[3], &control_points_vec[2])); // P2
            assert!(cpoints_are_close(&result_slice[4], &control_points_vec[3])); // P3

            // IMPORTANT: Free the memory
            free_smoothed_path(smoothed_path);
        } else {
            // If path is empty (e.g. due to an error in test setup), ensure it's handled.
            if smoothed_path.len == 0 && smoothed_path.points.is_null() {
                // This is an expected outcome for error cases, but not for this specific test's valid inputs.
            } else {
                panic!(
                    "Path has inconsistent state: len={}, ptr_is_null={}",
                    smoothed_path.len,
                    smoothed_path.points.is_null()
                );
            }
        }
    }

    #[test]
    fn test_smooth_cursor_path_edge_cases() {
        let alpha = 0.5;
        let points_per_segment_vec: Vec<i64> = vec![2]; // Dummy, may not be used if not enough points
        let points_per_segment_ptr = points_per_segment_vec.as_ptr();
        let points_per_segment_len = points_per_segment_vec.len();

        // Null pointers
        let path_null = smooth_cursor_path(
            std::ptr::null(),
            0,
            points_per_segment_ptr,
            points_per_segment_len,
            alpha,
            0.0,
            0.0,
            0.0,
        );
        assert!(path_null.points.is_null() && path_null.len == 0);
        // No need to free, as it should be null.

        // Empty slices
        let empty_points_vec: Vec<CPoint> = Vec::new();
        let path_empty_slice = smooth_cursor_path(
            empty_points_vec.as_ptr(),
            0,
            points_per_segment_ptr,
            points_per_segment_len,
            alpha,
            0.0,
            0.0,
            0.0,
        );
        assert!(path_empty_slice.points.is_null() && path_empty_slice.len == 0);

        // Insufficient points for a segment
        let control_points_short_vec =
            [cp(0.0, 0.0, 0.0), cp(1.0, 1.0, 100.0), cp(2.0, 0.0, 200.0)]; // 3 points
        let path_insufficient = smooth_cursor_path(
            control_points_short_vec.as_ptr(),
            control_points_short_vec.len(),
            points_per_segment_ptr,
            points_per_segment_len,
            alpha,
            0.0,
            0.0,
            0.0,
        );
        assert!(path_insufficient.points.is_null() && path_insufficient.len == 0);

        // Mismatched frame_amount_slice length
        let control_points_valid_len_vec = [
            cp(0.0, 0.0, 0.0),
            cp(1.0, 1.0, 100.0),
            cp(2.0, 0.0, 200.0),
            cp(3.0, 1.0, 300.0),
        ]; // 4 points, 1 segment
        let mismatched_segment_counts: Vec<i64> = vec![2, 2]; // Expects 1 segment, given 2 counts
        let path_mismatch = smooth_cursor_path(
            control_points_valid_len_vec.as_ptr(),
            control_points_valid_len_vec.len(),
            mismatched_segment_counts.as_ptr(),
            mismatched_segment_counts.len(),
            alpha,
            0.0,
            0.0,
            0.0,
        );
        assert!(path_mismatch.points.is_null() && path_mismatch.len == 0);

        // num_points_for_this_segment = 0
        let control_points_for_zero_segment = [
            cp(0.0, 0.0, 0.0),
            cp(1.0, 1.0, 100.0),
            cp(2.0, 0.0, 200.0),
            cp(3.0, 1.0, 300.0),
        ]; // 1 segment P1-P2
        let points_per_segment_zero: Vec<i64> = vec![0];
        let path_zero_points = smooth_cursor_path(
            control_points_for_zero_segment.as_ptr(),
            control_points_for_zero_segment.len(),
            points_per_segment_zero.as_ptr(),
            points_per_segment_zero.len(),
            alpha,
            0.0,
            0.0,
            0.0,
        );
        // Your code currently adds no points if num_points_for_this_segment is 0.
        assert_eq!(path_zero_points.len, 0);
        assert!(!path_zero_points.points.is_null() || path_zero_points.len == 0); // Could be non-null if Vec was allocated then shrunk to 0
        if !path_zero_points.points.is_null() && path_zero_points.len == 0 {
            // If allocated but empty
            free_smoothed_path(path_zero_points);
        }
    }
}
