use core::ffi::{c_void};
use super::imports::memrchr;
use rusl_core::test;


test!("test_found" {
    unsafe {
        let buf = [10u8, 20, 30, 20, 40];
        let r = memrchr(buf.as_ptr() as *const c_void, 20, 5);
        assert!(!r.is_null());
        assert_eq!(*(r as *const u8) , 20);
    }
});

test!("test_not_found" {
    unsafe {
        let buf = [10u8, 20, 30];
        let r = memrchr(buf.as_ptr() as *const c_void, 99, 3);
        assert!(r.is_null());
    }
});
