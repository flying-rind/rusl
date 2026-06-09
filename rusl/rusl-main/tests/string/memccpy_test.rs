use core::ffi::{c_void};
use super::imports::memccpy;
use test_framework::test;


test!("test_copy_until_c" {
    {
        let src = [1u8, 2, 3, 4, 5]; let mut dst = [0u8; 5];
        let r = memccpy(dst.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void, 3, 5);
        assert!(!r.is_null());
        assert_eq!(dst[0], 1); assert_eq!(dst[1], 2); assert_eq!(dst[2], 3);
    }
});

test!("test_not_found" {
    {
        let src = [1u8, 2, 3]; let mut dst = [0u8; 5];
        let r = memccpy(dst.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void, 99, 3);
        assert!(r.is_null());
        assert_eq!(dst[..3], [1, 2, 3]);
    }
});
