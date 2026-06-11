//! snprintf 集成测试 — 测试可变参数格式化到字符串缓冲区

use core::ffi::c_char;
use super::imports::snprintf;
use test_framework::test;

test!("snprintf_zero_size" {
    unsafe {
        let fmt = b"test\0".as_ptr() as *const c_char;
        let result = snprintf(core::ptr::null_mut(), 0, fmt);
        assert_eq!(result, 4);
    }
});

test!("snprintf_simple_string" {
    unsafe {
        let fmt = b"ab\0".as_ptr() as *const c_char;
        let mut buf: [u8; 8] = [0; 8];
        let result = snprintf(buf.as_mut_ptr() as *mut c_char, 8, fmt);
        assert_eq!(result, 2);
        assert_eq!(buf[0], b'a');
        assert_eq!(buf[1], b'b');
        assert_eq!(buf[2], 0);
    }
});

test!("snprintf_truncation" {
    unsafe {
        let fmt = b"hello world\0".as_ptr() as *const c_char;
        let mut buf: [u8; 6] = [0; 6];
        let result = snprintf(buf.as_mut_ptr() as *mut c_char, 6, fmt);
        assert_eq!(result, 11);
        assert_eq!(buf[4], b'o');
        assert_eq!(buf[5], 0);
    }
});
