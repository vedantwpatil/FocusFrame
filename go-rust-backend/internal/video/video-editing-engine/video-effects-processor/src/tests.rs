use crate::{
    calculate_t_j, catmull_rom_spline, free_smoothed_path, interpolate_points, linspace,
    smooth_cursor_path, CPoint,
};

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
    let control_points_short_vec = [cp(0.0, 0.0, 0.0), cp(1.0, 1.0, 100.0), cp(2.0, 0.0, 200.0)]; // 3 points
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
