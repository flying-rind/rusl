use core::ffi::{c_char};
use super::imports::strncpy;
use test_framework::test;


test!("test_basic_copy" {
    unsafe {
        let src = b"hello\0"; let mut dst = [0u8; 10];
        let r = strncpy(dst.as_mut_ptr() as *mut c_char, src.as_ptr() as *const c_char, 10);
        assert_eq!(r, dst.as_mut_ptr() as *mut c_char);
        assert_eq!(&dst[..6], b"hello\0");
    }
});

test!("test_truncated" {
    unsafe {
        let src = b"hello world\0"; let mut dst = [0xFFu8; 5];
        strncpy(dst.as_mut_ptr() as *mut c_char, src.as_ptr() as *const c_char, 5);
        assert_eq!(&dst[..5], b"hello");
    }
});
