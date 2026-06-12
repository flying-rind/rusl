//! printf — 格式化输出到标准输出。
//! 对应 musl src/stdio/printf.c
//!
//! 使用 c_variadic 直接提取 va_list, 委托给 vprintf。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use super::vprintf::vprintf;
use core::ffi::{c_char, c_int};

/// 将格式化字符串输出到 stdout。
#[no_mangle]
pub unsafe extern "C" fn printf(fmt: *const c_char, mut args: ...) -> c_int {
    let ap = &raw mut args as *mut VaList;
    vprintf(fmt, ap)
}
