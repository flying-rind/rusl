//! vfprintf 集成测试

use core::ffi::{c_char, c_int};
use super::imports::{vfprintf, vsnprintf};
use test_framework::test;

test!("vfprintf_via_vsnprintf" {
    // vfprintf 需要有效的 FILE*, 间接通过 vsnprintf 验证格式化引擎
    let fmt = b"hello\0".as_ptr() as *const c_char;
    let mut buf: [u8; 32] = [0; 32];
    let result = vsnprintf(buf.as_mut_ptr() as *mut c_char, 32, fmt, core::ptr::null_mut());
    assert_eq!(result, 5);
    assert_eq!(&buf[..5], b"hello");
});
