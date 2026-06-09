use core::ffi::{c_char};
use super::imports::strspn;
use test_framework::test;


test!("test_basic" {
    {
        let s = b"abcde\0"; let accept = b"abc\0";
        let r = strspn(s.as_ptr() as *const c_char, accept.as_ptr() as *const c_char);
        assert_eq!(r, 3);
    }
});

test!("test_no_match" {
    {
        let s = b"xyz\0"; let accept = b"abc\0";
        let r = strspn(s.as_ptr() as *const c_char, accept.as_ptr() as *const c_char);
        assert_eq!(r, 0);
    }
});
