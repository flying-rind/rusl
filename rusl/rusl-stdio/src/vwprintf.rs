//! vwprintf — 宽字符标准输出格式化（va_list 版本）。
//! 对应 musl src/stdio/vwprintf.c
//!
//! 直接委托 vfwprintf(stdout, _: ...) 实现，纯转发代理。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_int, c_uint};

/// vwprintf — 将格式化宽字符串输出到 stdout。
///
/// - `fmt`: 宽字符格式字符串（const wchar_t *）
/// - `ap`: va_list 参数列表
///
/// 返回值：成功时返回写入的宽字符总数；失败时返回 -1。
#[no_mangle]
pub extern "C" fn vwprintf(fmt: *const c_uint, ap: *mut VaList) -> c_int {
    loop {}
}
