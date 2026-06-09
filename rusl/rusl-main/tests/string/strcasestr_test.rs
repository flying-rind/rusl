use core::ffi::{c_char};
use super::imports::strcasestr;
use test_framework::test;


test!("test_found" {
    {
        let h = b"Hello World\0"; let n = b"world\0";
        let r = strcasestr(h.as_ptr() as *const c_char, n.as_ptr() as *const c_char);
        assert!(!r.is_null());
    }
});
