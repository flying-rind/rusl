use core::ffi::{c_char};
use super::imports::strrchr;
use test_framework::test;


test!("test_found" {
    unsafe {
        let s = b"hello world\0";
        let r = strrchr(s.as_ptr() as *const c_char, 'o' as i32);
        assert!(!r.is_null());
        assert_eq!(*(r as *const u8) , b'o');
    }
});

test!("test_not_found" {
    {
        let s = b"hello\0";
        let r = strrchr(s.as_ptr() as *const c_char, 'x' as i32);
        assert!(r.is_null());
    }
});
