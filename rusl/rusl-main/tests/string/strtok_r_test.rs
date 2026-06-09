use core::ffi::{c_char};
use super::imports::strtok_r;
use test_framework::test;


test!("test_first_token" {
    unsafe {
        let mut buf = *b"hello world\0";
        let sep = b" \0";
        let mut state: *mut c_char = core::ptr::null_mut();
        let r = strtok_r(buf.as_mut_ptr() as *mut c_char, sep.as_ptr() as *const c_char, &mut state);
        assert!(!r.is_null());
    }
});
