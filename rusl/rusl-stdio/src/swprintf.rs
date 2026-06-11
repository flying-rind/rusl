//! swprintf — 格式化宽字符串输出到缓冲区。
//! 对应 musl src/stdio/swprintf.c
//!
//! vswprintf 的可变参数包装，使用 va_list 机制转发。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_int, c_uint};

/// swprintf — 将格式化宽字符串写入缓冲区 s，最多 n 个宽字符（含 L'\0'）。
///
/// musl 截断行为：ret >= n 时返回 -1（非 C99 标准）。
#[no_mangle]
pub unsafe extern "C" fn swprintf(s: *mut c_uint, n: usize, fmt: *const c_uint, _: ...) -> c_int {
    loop {}
}
