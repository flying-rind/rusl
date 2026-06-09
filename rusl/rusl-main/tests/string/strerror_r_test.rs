use core::ffi::{c_char};
use super::imports::strerror_r;
use test_framework::test;


test!("test_valid_error" {
    {
        let mut buf = [0u8; 256];
        let r = strerror_r(0, buf.as_mut_ptr() as *mut c_char, 256);
        assert_eq!(r, 0);
    }
});
