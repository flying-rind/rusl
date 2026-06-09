use core::ffi::{c_char};
use super::imports::index;
use test_framework::test;


test!("test_found" {
    unsafe {
        let s = b"hello\0";
        let r = index(s.as_ptr() as *const c_char, 'l' as i32);
        assert!(!r.is_null());
    }
});
