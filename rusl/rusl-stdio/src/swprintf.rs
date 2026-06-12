//! swprintf — 格式化宽字符串输出到缓冲区。
//! 对应 musl src/stdio/swprintf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// swprintf — 将格式化宽字符串写入缓冲区 s，最多 n 个宽字符（含 L'\0'）。
#[no_mangle]
pub unsafe extern "C" fn swprintf(s: *mut c_int, n: usize, fmt: *const c_int, mut args: ...) -> c_int {
    let ap = &raw mut args as *mut VaList;
    super::vswprintf::vswprintf(s, n, fmt, ap)
}
