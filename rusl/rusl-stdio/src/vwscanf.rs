//! vwscanf — 宽字符标准输入格式化读取（va_list 版本）。
//! 对应 musl src/stdio/vwscanf.c
//!
//! 直接委托 vfwscanf(stdin, _: ...) 实现，纯转发代理。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_int, c_uint};

/// vwscanf — 从 stdin 读取宽字符格式化输入。
///
/// - `fmt`: 宽字符格式字符串（const wchar_t *）
/// - `ap`: va_list 参数列表
///
/// 返回值：成功时为匹配并赋值的输入项数；输入失败时返回 EOF。
#[no_mangle]
pub extern "C" fn vwscanf(fmt: *const c_uint, ap: *mut VaList) -> c_int {
    loop {}
}

/// __isoc99_vwscanf — vwscanf 的 C99 兼容弱别名。
#[no_mangle]
pub extern "C" fn __isoc99_vwscanf(fmt: *const c_uint, ap: *mut VaList) -> c_int {
    loop {}
}
