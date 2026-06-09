use core::ffi::{c_void};
use super::imports::memmove;
use test_framework::test;


test!("test_basic_copy" {
    unsafe {
        let src = [1u8, 2, 3, 4, 5];
        let mut dst = [0u8; 5];
        let result = memmove(dst.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void, 5);
        assert_eq!(result, dst.as_mut_ptr() as *mut c_void);
        assert_eq!(dst, [1, 2, 3, 4, 5]);
    }
});

test!("test_zero_length" {
    unsafe {
        let src = [1u8; 10]; let mut dst = [0u8; 10];
        memmove(dst.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void, 0);
        assert_eq!(dst, [0u8; 10]);
    }
});

test!("test_overlap" {
    unsafe {
        let mut buf = [1u8, 2, 3, 4, 5];
        let dst = buf.as_mut_ptr().add(2) as *mut c_void;
        let src = buf.as_ptr() as *const c_void;
        memmove(dst, src, 3);
        assert_eq!(buf, [1, 2, 1, 2, 3]);
    }
});
