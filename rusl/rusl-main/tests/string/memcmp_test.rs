use core::ffi::{c_void};
use super::imports::memcmp;
use test_framework::test;


test!("test_equal" {
    {
        let a = [1u8, 2, 3]; let b = [1u8, 2, 3];
        let r = memcmp(a.as_ptr() as *const c_void, b.as_ptr() as *const c_void, 3);
        assert_eq!(r, 0);
    }
});

test!("test_less_than" {
    {
        let a = [1u8, 2, 3]; let b = [1u8, 2, 4];
        let r = memcmp(a.as_ptr() as *const c_void, b.as_ptr() as *const c_void, 3);
        assert!(r < 0);
    }
});

test!("test_zero_length" {
    {
        let a = [1u8]; let b = [2u8];
        let r = memcmp(a.as_ptr() as *const c_void, b.as_ptr() as *const c_void, 0);
        assert_eq!(r, 0);
    }
});
