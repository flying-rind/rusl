use core::ffi::{c_void};
use super::imports::memcpy;
use test_framework::test;


test!("test_basic_copy" {
    unsafe {
        let src = [1u8, 2, 3, 4, 5];
        let mut dst = [0u8; 5];
        let result = memcpy(dst.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void, 5);
        assert_eq!(result, dst.as_mut_ptr() as *mut c_void);
        assert_eq!(dst, [1, 2, 3, 4, 5]);
    }
});

test!("test_zero_length" {
    unsafe {
        let src = [1u8; 10];
        let mut dst = [0u8; 10];
        let result = memcpy(dst.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void, 0);
        assert_eq!(result, dst.as_mut_ptr() as *mut c_void);
        assert_eq!(dst, [0u8; 10]);
    }
});

test!("test_single_byte" {
    unsafe {
        let src: [u8; 1] = [0xFF];
        let mut dst = [0u8; 1];
        memcpy(dst.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void, 1);
        assert_eq!(dst[0], 0xFF);
    }
});
