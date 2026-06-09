use super::imports::stpncpy;

use core::ffi::c_char;
use test_framework::test;


test!("test_basic_copy" {
    unsafe {
        let src = b"hello\0";
        let mut dst = [0u8; 10];
        let r = stpncpy(
            dst.as_mut_ptr() as *mut c_char,
            src.as_ptr() as *const c_char,
            10,
        );
        let expected = dst.as_mut_ptr().add(5) as *mut c_char;
        assert_eq!(r, expected);
    }
});
