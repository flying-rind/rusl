use core::ffi::{c_void};
use super::imports::bcmp;
use test_framework::test;

test!("test_equal" {
    unsafe {
        let a = [1u8, 2, 3]; let b = [1u8, 2, 3];
        let r = bcmp(a.as_ptr() as *const c_void, b.as_ptr() as *const c_void, 3);
        assert_eq!(r, 0);
    }
});

test!("test_not_equal" {
    unsafe {
        let a = [1u8, 2, 3]; let b = [1u8, 2, 4];
        let r = bcmp(a.as_ptr() as *const c_void, b.as_ptr() as *const c_void, 3);
        assert_ne!(r, 0);
    }
});
