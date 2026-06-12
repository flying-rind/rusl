//! 宽字符免锁 I/O 集成测试
//!
//! fgetwc_unlocked / getwc_unlocked / getwchar_unlocked
//! fputwc_unlocked / putwc_unlocked / putwchar_unlocked
//! fgetws_unlocked / fputws_unlocked
//!
//! musl 的宽字符 unlocked 变体不检查 NULL FILE*, 跳过 NULL 测试。

use core::ffi::c_int;
use super::imports::{
    fopen, fclose,
    fputwc_unlocked, putwchar_unlocked,
    fgetws_unlocked, fputws_unlocked,
};
use test_framework::test;

fn cstr(s: &[u8]) -> *const core::ffi::c_char {
    s.as_ptr() as *const core::ffi::c_char
}

// getwchar_unlocked 从 stdin 读取, 无输入时阻塞, 跳过

// ---- fputwc_unlocked ----

test!("fputwc_unlocked_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let ret = fputwc_unlocked(b'A' as c_int, f);
    let _ = ret;
    fclose(f);
});

// ---- putwchar_unlocked ----

test!("putwchar_unlocked_smoke" {
    let _ = putwchar_unlocked(b'Z' as c_int);
});

// ---- fgetws_unlocked ----

test!("fgetws_unlocked_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let mut buf: [c_int; 16] = [0; 16];
    let ret = fgetws_unlocked(buf.as_mut_ptr(), 16, f);
    assert!(ret.is_null());
    fclose(f);
});

// ---- fputws_unlocked ----

test!("fputws_unlocked_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let ws: [c_int; 2] = [0x41, 0];
    let ret = fputws_unlocked(ws.as_ptr(), f);
    assert!(ret >= 0);
    fclose(f);
});
