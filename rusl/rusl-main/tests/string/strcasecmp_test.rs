use core::ffi::{c_char};
use super::imports::strcasecmp;
use rusl_core::test;


test!("test_equal" {
    unsafe {
        let a = b"Hello\0"; let b = b"hello\0";
        let r = strcasecmp(a.as_ptr() as *const c_char, b.as_ptr() as *const c_char);
        assert_eq!(r, 0);
    }
});
