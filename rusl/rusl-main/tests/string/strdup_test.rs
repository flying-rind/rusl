use core::ffi::{c_char};
use super::imports::strdup;
use rusl_core::test;


test!("test_basic_dup" {
    unsafe {
        let s = b"hello\0";
        let r = strdup(s.as_ptr() as *const c_char);
        assert!(!r.is_null());
    }
});
