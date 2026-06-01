use core::ffi::{c_char};
use super::imports::strncmp;
use rusl_core::test;


test!("test_equal" {
    unsafe {
        let a = b"hello\0"; let b = b"hello\0";
        let r = strncmp(a.as_ptr() as *const c_char, b.as_ptr() as *const c_char, 5);
        assert_eq!(r, 0);
    }
});

test!("test_limited_n" {
    unsafe {
        let a = b"abcXXX\0"; let b = b"abcYYY\0";
        let r = strncmp(a.as_ptr() as *const c_char, b.as_ptr() as *const c_char, 3);
        assert_eq!(r, 0);
    }
});
