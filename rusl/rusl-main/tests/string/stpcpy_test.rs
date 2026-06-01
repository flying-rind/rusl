use core::ffi::{c_char};
use super::imports::stpcpy;
use rusl_core::test;


test!("test_basic_copy" {
    unsafe {
        let src = b"hello\0"; let mut dst = [0u8; 10];
        let r = stpcpy(dst.as_mut_ptr() as *mut c_char, src.as_ptr() as *const c_char);
        let expected = dst.as_mut_ptr().add(5) as *mut c_char;
        assert_eq!(r, expected);
        assert_eq!(&dst[..6], b"hello\0");
    }
});
