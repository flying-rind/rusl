use core::ffi::{c_void};
use super::imports::mempcpy;
use rusl_core::test;


test!("test_basic_copy" {
    unsafe {
        let src = [1u8, 2, 3, 4, 5]; let mut dst = [0u8; 5];
        let r = mempcpy(dst.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void, 5);
        let expected = dst.as_mut_ptr().add(5) as *mut c_void;
        assert_eq!(r, expected);
        assert_eq!(dst, [1, 2, 3, 4, 5]);
    }
});
