use core::ffi::{c_char};
use super::imports::strnlen;
use rusl_core::test;


test!("test_basic" {
    unsafe {
        let s = b"hello\0";
        let r = strnlen(s.as_ptr() as *const c_char, 10);
        assert_eq!(r, 5);
    }
});

test!("test_limited" {
    unsafe {
        let s = b"hello world\0";
        let r = strnlen(s.as_ptr() as *const c_char, 5);
        assert_eq!(r, 5);
    }
});
