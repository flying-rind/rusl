//! vsnprintf 集成测试

use core::ffi::c_char;
use super::imports::vsnprintf;
use test_framework::test;

test!("vsnprintf_zero_size" {
    let fmt = b"abc\0".as_ptr() as *const c_char;
    let result = vsnprintf(core::ptr::null_mut(), 0, fmt, core::ptr::null_mut());
    assert_eq!(result, 3);
});

test!("vsnprintf_simple_string" {
    let fmt = b"hello\0".as_ptr() as *const c_char;
    let mut buf: [u8; 16] = [0; 16];
    let result = vsnprintf(buf.as_mut_ptr() as *mut c_char, 16, fmt, core::ptr::null_mut());
    assert_eq!(result, 5);
    assert_eq!(buf[0], b'h');
    assert_eq!(buf[4], b'o');
    assert_eq!(buf[5], 0);
});

test!("vsnprintf_truncation" {
    let fmt = b"hello world\0".as_ptr() as *const c_char;
    let mut buf: [u8; 6] = [0; 6];
    let result = vsnprintf(buf.as_mut_ptr() as *mut c_char, 6, fmt, core::ptr::null_mut());
    assert_eq!(result, 11);
    assert_eq!(buf[4], b'o');
    assert_eq!(buf[5], 0);
});
