//! vfscanf / vscanf / vsscanf 集成测试
//!
//! va_list 版本的格式化输入。通过 vsscanf 字符串扫描验证基础行为。

use core::ffi::c_char;
use super::imports::{VaList, vsscanf, vfscanf, fopen, fclose};
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

fn zero_va_list() -> VaList {
    VaList {
        gp_offset: 0,
        fp_offset: 0,
        overflow_arg_area: core::ptr::null_mut(),
        reg_save_area: core::ptr::null_mut(),
    }
}

// ---- vsscanf 测试 ----

test!("vsscanf_basic" {
    let mut ap = zero_va_list();
    let ret = vsscanf(
        cstr(b"hello\0"),
        cstr(b"%*s\0"),
        &mut ap,
    );
    assert_eq!(ret, 0, "仅跳过的扫描应返回 0");
});

test!("vsscanf_empty_fmt" {
    let mut ap = zero_va_list();
    let ret = vsscanf(
        cstr(b"hello\0"),
        cstr(b"\0"),
        &mut ap,
    );
    assert_eq!(ret, 0, "空格式字符串应返回 0");
});

// 跳过: vsscanf 的 %d 需要从 va_list 读取 int*, 空 va_list 不安全

// ---- vfscanf 测试 ----

// 跳过: vfscanf 的 %d 需要从 va_list 读取 int*, 空 va_list 不安全
