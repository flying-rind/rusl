//! 宽字符 I/O 集成测试
//!
//! fgetwc / fputwc / getwc / putwc / getwchar / putwchar
//! ungetwc / fgetws / fputws
//!
//! 宽字符函数使用 wchar_t (c_int) 作为字符单元。
//! musl 的宽字符函数不检查 NULL FILE*/NULL buf, 跳过 NULL 测试。

use core::ffi::c_int;
use super::imports::{
    fopen, fclose,
    fgetwc, fputwc,
    putwchar,
    ungetwc,
    fgetws, fputws,
};
use test_framework::test;

fn cstr(s: &[u8]) -> *const core::ffi::c_char {
    s.as_ptr() as *const core::ffi::c_char
}

// ---- fgetwc 测试 ----

test!("fgetwc_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = fgetwc(f);
    assert_eq!(ret, -1, "/dev/null 的 fgetwc 应返回 WEOF");
    fclose(f);
});

// ---- fputwc 测试 ----

test!("fputwc_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let ret = fputwc(b'X' as c_int, f);
    let _ = ret;
    fclose(f);
});

// ---- putwchar ----
// getwchar 从 stdin 读取, 无输入时阻塞, 跳过

test!("putwchar_smoke" {
    let _ = putwchar(b'Z' as c_int);
});

// ---- ungetwc 测试 ----

test!("ungetwc_wEOF" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = ungetwc(-1, f);
    assert_eq!(ret, -1, "ungetwc(WEOF) 应返回 WEOF");
    fclose(f);
});

// ---- fgetws 测试 ----

test!("fgetws_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let mut buf: [c_int; 16] = [0; 16];
    let ret = fgetws(buf.as_mut_ptr(), 16, f);
    assert!(ret.is_null(), "/dev/null 的 fgetws 应返回 NULL");
    fclose(f);
});

// ---- fputws 测试 ----

test!("fputws_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let ws: [c_int; 2] = [0x41, 0]; // L"A\0"
    let ret = fputws(ws.as_ptr(), f);
    assert!(ret >= 0, "fputws 应返回 >= 0");
    fclose(f);
});
