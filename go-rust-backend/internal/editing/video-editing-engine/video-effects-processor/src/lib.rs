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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
