use core::ffi::{c_void};
use super::imports::bcopy;
use test_framework::test;


test!("test_basic_copy" {
    unsafe {
        let src = [1u8, 2, 3, 4, 5]; let mut dst = [0u8; 5];
        bcopy(src.as_ptr() as *const c_void, dst.as_mut_ptr() as *mut c_void, 5);
        assert_eq!(dst, [1, 2, 3, 4, 5]);
    }
});

test!("test_zero_length" {
    unsafe {
        let src = [1u8; 5]; let mut dst = [0u8; 5];
        bcopy(src.as_ptr() as *const c_void, dst.as_mut_ptr() as *mut c_void, 0);
        assert_eq!(dst, [0u8; 5]);
    }
});
