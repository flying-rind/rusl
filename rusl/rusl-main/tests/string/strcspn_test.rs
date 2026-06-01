use core::ffi::{c_char};
use super::imports::strcspn;
use rusl_core::test;


test!("test_basic" {
    unsafe {
        let s = b"abcde\0"; let reject = b"de\0";
        let r = strcspn(s.as_ptr() as *const c_char, reject.as_ptr() as *const c_char);
        assert_eq!(r, 3);
    }
});

test!("test_no_match" {
    unsafe {
        let s = b"abc\0"; let reject = b"xyz\0";
        let r = strcspn(s.as_ptr() as *const c_char, reject.as_ptr() as *const c_char);
        assert_eq!(r, 3);
    }
});
