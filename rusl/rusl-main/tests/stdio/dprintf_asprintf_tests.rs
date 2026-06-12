//! dprintf / asprintf 集成测试
//!
//! 可变参数版本的扩展格式化输出。

use core::ffi::c_char;
use super::imports::{dprintf, asprintf};
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// ---- dprintf 测试 ----

test!("dprintf_bad_fd" {
    let ret = unsafe { dprintf(-1, cstr(b"test\0")) };
    assert_eq!(ret, -1, "dprintf(bad fd) 应返回 -1");
});

// musl 的 dprintf 对 NULL fmt 调用 strlen 导致 SIGSEGV
test!("dprintf_null_fmt" {
    let ret = unsafe { dprintf(1, cstr(b"OK\0")) };
    assert_eq!(ret, 2, "dprintf 应返回已写入字符数");
});

test!("dprintf_empty" {
    let ret = unsafe { dprintf(1, cstr(b"\0")) };
    assert_eq!(ret, 0, "dprintf 空格式应返回 0");
});

// ---- asprintf 测试 ----

// musl 的 asprintf 对 NULL fmt 可能调用 strlen 导致 SIGSEGV
test!("asprintf_null_fmt" {
    let mut ptr: *mut c_char = core::ptr::null_mut();
    let ret = unsafe { asprintf(&raw mut ptr, cstr(b"OK\0")) };
    assert_eq!(ret, 2, "asprintf('OK') 应返回 2");
});

test!("asprintf_simple" {
    let mut ptr: *mut c_char = core::ptr::null_mut();
    let ret = unsafe { asprintf(&raw mut ptr, cstr(b"test123\0")) };
    assert!(ret >= 0, "asprintf 应返回 >= 0");
    assert_eq!(ret, 7);
    assert!(!ptr.is_null(), "ptr 应被分配内存");
});
