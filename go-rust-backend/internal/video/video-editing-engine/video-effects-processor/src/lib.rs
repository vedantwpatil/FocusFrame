#[no_mangle]
pub extern "C" fn process_image_data_rust(data_ptr: *mut u8, len: u32) {
    // Unsafe is needed bcause we are dealing with raw pointers from C/Go
    unsafe {
        let data_slice = std::slice::from_raw_parts_mut(data_ptr, len as usize);

        // Modify the first byte if the slice isn't empty
        if !data_slice.is_empty() {
            data_slice[0] = data_slice[0].wrapping_add(10);
        }
    }

    println!("[Rust] process_image_data_rust called, first byte modified (if present).");
}

// Need this to allow for go code to be able to use this function
#[no_mangle]
pub extern "C" fn add(left: i32, right: i32) -> i32 {
    left + right
}

#[no_mangle]
pub extern "C" fn greet_from_rust() {
    println!("Hello from Rust!");
}

#[repr(C)]
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

fn num_segments(point_chain: &[CPoint], quadruple_size: usize) -> usize {
    return point_chain.len() - (quadruple_size - 1);
}

fn catmull_rom_spline(
    p0: CPoint,
    p1: CPoint,
    p2: CPoint,
    p3: CPoint,
    num_points: usize,
    alpha: f32,
) {
    let t_0: f32 = 0.0;
    let t_1 = calculate_t_j(t_0, &p0, &p1, alpha);
    let t_2 = calculate_t_j(t_1, &p1, &p2, alpha);
    let t_3 = calculate_t_j(t_2, &p2, &p3, alpha);

    let t = reshape_to_column_vector_f32(linspace(t_1, t_2, num_points));
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
fn reshape_to_column_vector_f32(flat_vector: Vec<f32>) -> Vec<Vec<f32>> {
    let mut reshaped_vector = Vec::with_capacity(flat_vector.len());
    for val in flat_vector {
        reshaped_vector.push(vec![val]);
    }
    reshaped_vector
}
fn calculate_t_j(t_i: f32, p_i: &CPoint, p_j: &CPoint, alpha: f32) -> f32 {
    let x_i = p_i.x;
    let y_i = p_i.y;

    let x_j = p_j.x;
    let y_j = p_j.y;

    let dx = x_j - x_i;
    let dy = y_j - y_i;

    let l = (dx.powi(2) + dy.powi(2)).sqrt();
    return t_i + l.powf(alpha);
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
