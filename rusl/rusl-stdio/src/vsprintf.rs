//! vsprintf — 字符串格式化输出（va_list 版本，无边界检查）。
//! 对应 musl src/stdio/vsprintf.c
//!
//! 通过将 INT_MAX 作为 size 参数传入 vsnprintf 实现。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use super::vsnprintf::vsnprintf;
use core::ffi::{c_char, c_int};

/// vsprintf — 将格式化字符串写入用户缓冲区（无边界检查）。
///
/// 行为等价于 vsnprintf(s, INT_MAX, fmt, ap)。
#[no_mangle]
pub extern "C" fn vsprintf(s: *mut c_char, fmt: *const c_char, ap: *mut VaList) -> c_int {
    vsnprintf(s, c_int::MAX as usize, fmt, ap)
}
