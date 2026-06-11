//! vsprintf — 字符串格式化输出（va_list 版本，无边界检查）。
//! 对应 musl src/stdio/vsprintf.c
//!
//! 通过将 INT_MAX 作为 size 参数传入 vsnprintf 实现。纯转发代理。
//! 调用者必须保证缓冲区足够大。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;
use core::ffi::c_int;

/// vsprintf — 将格式化字符串写入用户缓冲区（无边界检查）。
///
/// - `s`: 输出缓冲区（调用者保证足够大）
/// - `fmt`: 格式字符串
/// - `ap`: va_list 参数列表
///
/// 返回值：成功时返回写入的字符总数（不含 '\0'）；失败时返回负值。
/// 行为等价于 vsnprintf(s, INT_MAX, fmt, ap)。
#[no_mangle]
pub extern "C" fn vsprintf(s: *mut c_char, fmt: *const c_char, ap: *mut VaList) -> c_int {
    unimplemented!()
}
