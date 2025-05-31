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
    pub x: f64,
    pub y: f64,
    pub timestamp_ms: i64,
}

#[repr(C)]
pub struct CSmoothedPath {
    pub points: *mut CPoint, // Pointer to an array of CPoint
    pub len: usize,          // Number of points in the array
}

#[no_mangle]
pub extern "C" fn smooth_cursor_path(
    raw_points_ptr: *const CPoint,
    num_points: usize,
    tension: f64,
    friction: f64,
    mass: f64,
) -> CSmoothedPath {
    // 1. Convert raw_points_ptr and num_points into a Rust slice:
    //    let raw_points = unsafe { std::slice::from_raw_parts(raw_points_ptr, num_points) };
    // 2. Implement your smoothing logic using these raw_points and parameters.
    // 3. Allocate memory for the smoothed points that Rust will own.
    //    IMPORTANT: You must also provide a function to free this memory from Go.
    //    let mut smoothed_vec: Vec<CPoint> = ...; // Your smoothed points
    //    smoothed_vec.shrink_to_fit(); // Optional
    //    let path = CSmoothedPath {
    //        points: smoothed_vec.as_mut_ptr(),
    //        len: smoothed_vec.len(),
    //    };
    //    std::mem::forget(smoothed_vec); // Prevent Rust from dropping the Vec's data, as Go will manage it via the pointer
    //    path
    // Placeholder:
    CSmoothedPath {
        points: std::ptr::null_mut(),
        len: 0,
    }
}

#[no_mangle]
pub extern "C" fn free_smoothed_path(path: CSmoothedPath) {
    // unsafe {
    //     if !path.points.is_null() {
    //         let _ = Vec::from_raw_parts(path.points, path.len, path.len); // Reconstruct Vec to deallocate
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
