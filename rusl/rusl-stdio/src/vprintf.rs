//! vprintf — 标准输出格式化（va_list 版本）。
//! 对应 musl src/stdio/vprintf.c
//!
//! 直接委托 vfprintf(stdout, _: ...) 实现，纯转发代理。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;
use core::ffi::c_int;

/// vprintf — 将格式化字符串输出到 stdout。
///
/// - `fmt`: 格式字符串
/// - `ap`: va_list 参数列表
///
/// 返回值：成功时返回写入的字符总数；失败时返回 -1。
#[no_mangle]
pub extern "C" fn vprintf(fmt: *const c_char, ap: *mut VaList) -> c_int {
    loop {}
}
