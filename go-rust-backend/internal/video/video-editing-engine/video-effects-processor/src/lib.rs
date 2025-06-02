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
    let weight1 = (t_end - t) / (t_end - t_start);
    let weight2 = (t - t_start) / (t_end - t_start);

    CPoint {
        x: weight1 * p_start.x + weight2 * p_end.x,
        y: weight1 * p_start.y + weight2 * p_end.y,
        timestamp_ms: -1.0,
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
    num_points: usize,
    tension: f32,
    friction: f32,
    mass: f32,
) -> CSmoothedPath {
    // Ensure the pointer is not null before trying to create a slice from it
    if raw_points_ptr.is_null() || num_points == 0 {
        return CSmoothedPath {
            points: std::ptr::null_mut(),
            len: 0,
        };
    }

    // Unsafe block is required because we are dereferencing a raw pointer
    // and trusting that I wrote the code correctly and have provided a valid pointer and length
    let points_slice: &[CPoint] = unsafe { std::slice::from_raw_parts(raw_points_ptr, num_points) };

    if points_slice.is_empty() {
        return CSmoothedPath {
            points: std::ptr::null_mut(),
            len: 0,
        };
    }

    // Smoothing Logic
    // 1. Read data from `points_slice`.
    // 2. Perform calculations (Centrpetal ron catmull spline interpretation for path generation, physics based mouse movement).
    // 3. Allocate new memory for the smoothed points (e.g., using Vec<CPoint>).
    // 4. Populate this new memory with the smoothed CPoint data.
    // 5. Convert the Vec<CPoint> into a raw pointer and length to return in CSmoothedPath.
    //    Remember to use std::mem::forget on the Vec to prevent Rust from deallocating
    //    the memory if Go is supposed to manage it via the returned pointer.

    let quadruple_size: usize = 4;
    // This effects how the smoothness of the lines, either choose 0.5 or 1.0
    // TODO: Make this personally configurable for the user to choose which they prefer more and
    // make it a sliding value between 0.0-1.0
    let alpha = 0.5;

    let smooth = catmull_rom_chain(points_slice, num_points, alpha, quadruple_size);

    // Placeholder for actual smoothed path generation
    let mut smoothed_points_vec: Vec<CPoint> = Vec::new();
    smoothed_points_vec.push(points_slice[0]);

    // Convert Vec to CSmoothedPath for returning to C/Go
    // This leaks the memory, which Go will need to manage and free later
    // using `free_smoothed_path`.
    smoothed_points_vec.shrink_to_fit(); // Reduces the capcity to the length
    let len = smoothed_points_vec.len();
    let ptr = smoothed_points_vec.as_mut_ptr();
    std::mem::forget(smoothed_points_vec); // Prevent Rust from dropping the data

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
