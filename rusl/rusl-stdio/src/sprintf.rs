//! sprintf — 格式化输出到字符串缓冲区（无边界检查）。
//! 对应 musl src/stdio/sprintf.c
//!
//! 使用 c_variadic 直接提取 va_list, 委托给 vsprintf。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use super::vsprintf::vsprintf;
use core::ffi::{c_char, c_int};

/// 将格式化字符串写入用户提供的缓冲区 s（无边界检查）。
#[no_mangle]
pub unsafe extern "C" fn sprintf(s: *mut c_char, fmt: *const c_char, mut args: ...) -> c_int {
    let ap = &raw mut args as *mut VaList;
    vsprintf(s, fmt, ap)
}
