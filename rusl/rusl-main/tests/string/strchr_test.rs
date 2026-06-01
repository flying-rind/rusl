use core::ffi::{c_char};
use super::imports::strchr;
use rusl_core::test;


test!("test_found" {
    unsafe {
        let s = b"hello\0";
        let r = strchr(s.as_ptr() as *const c_char, 'l' as i32);
        assert!(!r.is_null());
        assert_eq!(*(r as *const u8) , b'l');
    }
});

test!("test_not_found" {
    unsafe {
        let s = b"hello\0";
        let r = strchr(s.as_ptr() as *const c_char, 'x' as i32);
        assert!(r.is_null());
    }
});

test!("test_find_null" {
    unsafe {
        let s = b"hello\0";
        let r = strchr(s.as_ptr() as *const c_char, 0);
        assert!(!r.is_null());
        assert_eq!(*(r as *const u8), 0);
    }
});
