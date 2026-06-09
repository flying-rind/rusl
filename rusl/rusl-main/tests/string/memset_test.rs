use core::ffi::{c_void};
use super::imports::memset;
use test_framework::test;


test!("test_basic_set" {
    {
        let mut buf = [0u8; 10];
        memset(buf.as_mut_ptr() as *mut c_void, 0xAB, 10);
        assert_eq!(buf, [0xABu8; 10]);
    }
});

test!("test_zero_length" {
    {
        let mut buf = [0xFFu8; 5];
        memset(buf.as_mut_ptr() as *mut c_void, 0x00, 0);
        assert_eq!(buf, [0xFFu8; 5]);
    }
});

test!("test_partial_set" {
    {
        let mut buf = [0u8; 10];
        memset(buf.as_mut_ptr() as *mut c_void, 0xFF, 5);
        // First 5 bytes set to 0xFF
        // Remaining bytes unchanged
    }
});
