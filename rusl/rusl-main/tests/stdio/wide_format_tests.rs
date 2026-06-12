//! 宽字符格式化输出集成测试
//!
//! wprintf / fwprintf / swprintf (可变参数版本)
//! 宽字符格式字符串使用 wchar_t* (c_int*) 而非 char*。
//! v*printf 需要有效的 va_list, 集成测试中跳过。

use core::ffi::{c_char, c_int};
use super::imports::{
    fopen, fclose, fflush,
    wprintf, fwprintf, swprintf,
};
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

fn to_wcs(s: &[u8]) -> ([c_int; 64], usize) {
    let mut arr = [0i32; 64];
    let len = s.len().min(63);
    for i in 0..len {
        arr[i] = s[i] as c_int;
    }
    arr[len] = 0;
    (arr, len)
}

// ---- wprintf 测试 ----

test!("wprintf_smoke" {
    let (wfmt, _) = to_wcs(b"test\n");
    let ret = unsafe { wprintf(wfmt.as_ptr()) };
    assert!(ret >= 0);
    fflush(core::ptr::null_mut());
});

// ---- fwprintf 测试 ----

test!("fwprintf_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let (wfmt, _) = to_wcs(b"hello");
    let ret = unsafe { fwprintf(f, wfmt.as_ptr()) };
    assert_eq!(ret, 5);
    fclose(f);
});

// ---- swprintf 测试 ----

test!("swprintf_basic" {
    let mut buf: [c_int; 32] = [0; 32];
    let (wfmt, _) = to_wcs(b"abc");
    let ret = unsafe { swprintf(buf.as_mut_ptr(), 32, wfmt.as_ptr()) };
    assert_eq!(ret, 3);
    assert_eq!(buf[0], 'a' as c_int);
    assert_eq!(buf[1], 'b' as c_int);
    assert_eq!(buf[2], 'c' as c_int);
    assert_eq!(buf[3], 0);
});
