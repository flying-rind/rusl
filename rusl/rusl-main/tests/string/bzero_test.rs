use core::ffi::{c_void};
use super::imports::bzero;
use rusl_core::test;


test!("test_basic_zero" {
    unsafe {
        let mut buf = [0xFFu8; 10];
        bzero(buf.as_mut_ptr() as *mut c_void, 10);
        assert_eq!(buf, [0u8; 10]);
    }
});

test!("test_zero_length" {
    unsafe {
        let mut buf = [0xFFu8; 5];
        bzero(buf.as_mut_ptr() as *mut c_void, 0);
        assert_eq!(buf, [0xFFu8; 5]);
    }
});
