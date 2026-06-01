use core::ffi::{c_void};
use super::imports::swab;
use rusl_core::test;


test!("test_basic_swap" {
    unsafe {
        let src = [1u8, 2, 3, 4, 5, 6]; let mut dst = [0u8; 6];
        swab(src.as_ptr() as *const c_void, dst.as_mut_ptr() as *mut c_void, 6);
        assert_eq!(dst, [2, 1, 4, 3, 6, 5]);
    }
});

test!("test_odd_length" {
    unsafe {
        let src = [1u8, 2, 3, 4, 5]; let mut dst = [0u8; 5];
        swab(src.as_ptr() as *const c_void, dst.as_mut_ptr() as *mut c_void, 5);
        assert_eq!(dst[0..4], [2, 1, 4, 3]);
        assert_eq!(dst[4], 0);
    }
});
