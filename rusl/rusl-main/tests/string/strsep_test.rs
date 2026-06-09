use core::ffi::{c_char};
use super::imports::strsep;
use test_framework::test;


test!("test_basic_token" {
    {
        let mut buf = *b"hello world\0";
        let mut ptr: *mut c_char = buf.as_mut_ptr() as *mut c_char;
        let sep = b" \0";
        let r = strsep(&mut ptr, sep.as_ptr() as *const c_char);
        assert!(!r.is_null());
    }
});
