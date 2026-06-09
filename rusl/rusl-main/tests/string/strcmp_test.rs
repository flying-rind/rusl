use core::ffi::{c_char};
use super::imports::strcmp;
use test_framework::test;


test!("test_equal" {
    unsafe {
        let a = b"hello\0"; let b = b"hello\0";
        let r = strcmp(a.as_ptr() as *const c_char, b.as_ptr() as *const c_char);
        assert_eq!(r, 0);
    }
});

test!("test_less" {
    unsafe {
        let a = b"abc\0"; let b = b"abd\0";
        let r = strcmp(a.as_ptr() as *const c_char, b.as_ptr() as *const c_char);
        assert!(r < 0);
    }
});
