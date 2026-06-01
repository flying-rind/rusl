use core::ffi::{c_char};
use super::imports::strtok;
use rusl_core::test;


test!("test_first_token" {
    unsafe {
        let mut buf = *b"hello world\0";
        let sep = b" \0";
        let r = strtok(buf.as_mut_ptr() as *mut c_char, sep.as_ptr() as *const c_char);
        assert!(!r.is_null());
    }
});
