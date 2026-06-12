//! 宽字符格式化输入集成测试
//!
//! wscanf / fwscanf / swscanf (可变参数版本)
//! v*scanf 需要有效的 va_list, 集成测试中跳过。

use core::ffi::c_int;
use super::imports::{
    fopen, fclose,
    fwscanf, swscanf,
};
use test_framework::test;

fn cstr(s: &[u8]) -> *const core::ffi::c_char {
    s.as_ptr() as *const core::ffi::c_char
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

// ---- fwscanf 测试 ----

test!("fwscanf_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let (wfmt, _) = to_wcs(b"%d");
    let ret = unsafe { fwscanf(f, wfmt.as_ptr()) };
    assert_eq!(ret, -1, "/dev/null 的 fwscanf 应返回 WEOF");
    fclose(f);
});

// ---- swscanf 测试 ----

test!("swscanf_empty_input" {
    let (wsrc, _) = to_wcs(b"");
    let (wfmt, _) = to_wcs(b"%d");
    let ret = unsafe { swscanf(wsrc.as_ptr(), wfmt.as_ptr()) };
    assert_eq!(ret, -1, "swscanf 空输入应返回 WEOF");
});
