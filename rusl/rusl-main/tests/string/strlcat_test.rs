use core::ffi::{c_char};
use super::imports::strlcat;
use rusl_core::test;


test!("test_basic_concat" {
    unsafe {
        let mut buf = [0u8; 20];
        core::ptr::copy_nonoverlapping(b"hello\0".as_ptr(), buf.as_mut_ptr(), 6);
        let src = b" world\0";
        let r = strlcat(buf.as_mut_ptr() as *mut c_char, src.as_ptr() as *const c_char, 20);
        assert_eq!(r, 11);
        assert_eq!(&buf[..12], b"hello world\0");
    }
});
