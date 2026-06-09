use core::ffi::{c_char};
use super::imports::strpbrk;
use test_framework::test;


test!("test_found" {
    unsafe {
        let s = b"hello world\0"; let accept = b"wr\0";
        let r = strpbrk(s.as_ptr() as *const c_char, accept.as_ptr() as *const c_char);
        assert!(!r.is_null());
        assert_eq!(*(r as *const u8), b'w');
    }
});

test!("test_not_found" {
    {
        let s = b"hello\0"; let accept = b"xyz\0";
        let r = strpbrk(s.as_ptr() as *const c_char, accept.as_ptr() as *const c_char);
        assert!(r.is_null());
    }
});
