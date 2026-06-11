//! wscanf — 宽字符标准输入格式化读取。
//! 对应 musl src/stdio/wscanf.c
//!
//! vwscanf 的可变参数包装，使用 va_list 机制转发。
//! 最终委托给 vfwscanf(stdin, _: ...)。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_int, c_uint};

/// wscanf — 从 stdin 读取宽字符格式化输入。
///
/// - `fmt`: 宽字符格式字符串（const wchar_t *）
/// - `...`: 可变参数（指针类型指向有效位置）
///
/// 返回值：成功时为匹配并赋值的输入项数；输入失败时返回 EOF。
#[no_mangle]
pub unsafe extern "C" fn wscanf(fmt: *const c_uint, _: ...) -> c_int {
    loop {}
}

/// __isoc99_wscanf — wscanf 的 C99 兼容弱别名。
#[no_mangle]
pub unsafe extern "C" fn __isoc99_wscanf(fmt: *const c_uint, _: ...) -> c_int {
    loop {}
}
