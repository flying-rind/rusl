use core::ffi::{c_char};
use super::imports::strstr;
use test_framework::test;


test!("test_found" {
    unsafe {
        let h = b"hello world\0"; let n = b"world\0";
        let r = strstr(h.as_ptr() as *const c_char, n.as_ptr() as *const c_char);
        assert!(!r.is_null());
    }
});

test!("test_not_found" {
    unsafe {
        let h = b"hello\0"; let n = b"xyz\0";
        let r = strstr(h.as_ptr() as *const c_char, n.as_ptr() as *const c_char);
        assert!(r.is_null());
    }
});

test!("test_empty_needle" {
    unsafe {
        let h = b"hello\0"; let n = b"\0";
        let r = strstr(h.as_ptr() as *const c_char, n.as_ptr() as *const c_char);
        assert_eq!(r, h.as_ptr() as *mut c_char);
    }
});
