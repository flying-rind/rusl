use core::ffi::{c_char};
use super::imports::strlen;
use test_framework::test;


test!("test_basic_length" {
    unsafe {
        let s = b"hello\0";
        let r = strlen(s.as_ptr() as *const c_char);
        assert_eq!(r, 5);
    }
});

test!("test_empty_string" {
    unsafe {
        let s = b"\0";
        let r = strlen(s.as_ptr() as *const c_char);
        assert_eq!(r, 0);
    }
});
