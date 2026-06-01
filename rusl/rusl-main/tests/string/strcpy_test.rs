use core::ffi::{c_char};
use super::imports::strcpy;
use rusl_core::test;


test!("test_basic_copy" {
    unsafe {
        let src = b"hello\0"; let mut dst = [0u8; 10];
        let r = strcpy(dst.as_mut_ptr() as *mut c_char, src.as_ptr() as *const c_char);
        assert_eq!(r, dst.as_mut_ptr() as *mut c_char);
        assert_eq!(&dst[..6], b"hello\0");
    }
});

test!("test_empty_string" {
    unsafe {
        let src = b"\0"; let mut dst = [0xFFu8; 5];
        strcpy(dst.as_mut_ptr() as *mut c_char, src.as_ptr() as *const c_char);
        assert_eq!(dst[0], 0);
    }
});
