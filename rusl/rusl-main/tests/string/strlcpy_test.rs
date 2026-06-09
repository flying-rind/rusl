use core::ffi::{c_char};
use super::imports::strlcpy;
use test_framework::test;


test!("test_basic_copy" {
    {
        let src = b"hello\0"; let mut dst = [0u8; 10];
        let r = strlcpy(dst.as_mut_ptr() as *mut c_char, src.as_ptr() as *const c_char, 10);
        assert_eq!(r, 5);
        assert_eq!(&dst[..6], b"hello\0");
    }
});

test!("test_truncated" {
    {
        let src = b"hello world\0"; let mut dst = [0xFFu8; 5];
        let r = strlcpy(dst.as_mut_ptr() as *mut c_char, src.as_ptr() as *const c_char, 5);
        assert_eq!(r, 11);
        assert_eq!(dst[4], 0);
    }
});
