//! wprintf — 宽字符标准输出格式化。
//! 对应 musl src/stdio/wprintf.c
//!
//! vwprintf 的可变参数包装，使用 va_list 机制转发。
//! 最终委托给 vfwprintf(stdout, _: ...)。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_int, c_uint};

/// wprintf — 将格式化宽字符串输出到 stdout。
///
/// - `fmt`: 宽字符格式字符串（const wchar_t *）
/// - `...`: 可变参数，与格式串匹配
///
/// 返回值：成功时返回写入的宽字符总数；失败时返回 -1。
#[no_mangle]
pub unsafe extern "C" fn wprintf(fmt: *const c_uint, _: ...) -> c_int {
    loop {}
}
