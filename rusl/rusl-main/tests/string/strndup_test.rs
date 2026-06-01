use core::ffi::{c_char};
use super::imports::strndup;
use rusl_core::test;


test!("test_basic_ndup" {
    unsafe {
        let s = b"hello world\0";
        let r = strndup(s.as_ptr() as *const c_char, 5);
        assert!(!r.is_null());
    }
});
