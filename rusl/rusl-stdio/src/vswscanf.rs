//! vswscanf — 宽字符串格式化输入（va_list 版本）。
//! 对应 musl src/stdio/vswscanf.c
//!
//! 通过创建自定义只读 FILE 流（wstring_read 回调 + wcsrtombs 惰性转换），
//! 将 vfwscanf 的输入源重定向到用户提供的宽字符串。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_int, c_uint};

/// vswscanf — 从宽字符串读取格式化输入。
///
/// - `s`: 源宽字符串（只读，const wchar_t *）
/// - `fmt`: 宽字符格式字符串（const wchar_t *）
/// - `ap`: va_list 参数列表
///
/// 返回值：成功时为匹配并赋值的输入项数；输入失败时返回 EOF。
#[no_mangle]
pub extern "C" fn vswscanf(
    s: *const c_uint,
    fmt: *const c_uint,
    ap: *mut VaList,
) -> c_int {
    unimplemented!()
}

/// __isoc99_vswscanf — vswscanf 的 C99 兼容弱别名。
#[no_mangle]
pub extern "C" fn __isoc99_vswscanf(
    s: *const c_uint,
    fmt: *const c_uint,
    ap: *mut VaList,
) -> c_int {
    unimplemented!()
}
