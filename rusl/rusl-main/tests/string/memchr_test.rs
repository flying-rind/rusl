use core::ffi::{c_void};
use super::imports::memchr;
use test_framework::test;


test!("test_found" {
    unsafe {
        let buf = [10u8, 20, 30, 40];
        let r = memchr(buf.as_ptr() as *const c_void, 30, 4);
        assert!(!r.is_null());
        assert_eq!( *(r as *const u8) , 30);
    }
});

test!("test_not_found" {
    {
        let buf = [10u8, 20, 30];
        let r = memchr(buf.as_ptr() as *const c_void, 99, 3);
        assert!(r.is_null());
    }
});

test!("test_zero_length" {
    {
        let buf = [10u8];
        let r = memchr(buf.as_ptr() as *const c_void, 10, 0);
        assert!(r.is_null());
    }
});
