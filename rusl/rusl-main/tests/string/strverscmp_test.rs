use core::ffi::{c_char};
use super::imports::strverscmp;
use test_framework::test;


test!("test_version_compare" {
    unsafe {
        let a = b"file1\0"; let b = b"file10\0";
        let r = strverscmp(a.as_ptr() as *const c_char, b.as_ptr() as *const c_char);
        assert!(r < 0);
    }
});
