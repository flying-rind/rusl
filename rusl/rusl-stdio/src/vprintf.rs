//! vprintf — 标准输出格式化（va_list 版本）。
//! 对应 musl src/stdio/vprintf.c
//!
//! 委托 vfprintf(stdout, fmt, ap) 实现。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use super::vfprintf::vfprintf;
use super::stdout::stdout;
use core::ffi::{c_char, c_int};

/// vprintf — 将格式化字符串输出到 stdout。
#[no_mangle]
pub extern "C" fn vprintf(fmt: *const c_char, ap: *mut VaList) -> c_int {
    unsafe { vfprintf(stdout, fmt, ap) }
}
