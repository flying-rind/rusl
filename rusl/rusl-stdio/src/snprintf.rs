//! snprintf — 格式化输出到定长缓冲区。
//! 对应 musl src/stdio/snprintf.c
//!
//! 使用 c_variadic 直接提取 va_list, 委托给 vsnprintf。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use super::vsnprintf::vsnprintf;
use core::ffi::{c_char, c_int};

/// 将格式化字符串写入定长缓冲区 s（最多 n 字节，含结尾 '\0'）。
#[no_mangle]
pub unsafe extern "C" fn snprintf(
    s: *mut c_char,
    n: usize,
    fmt: *const c_char,
    mut args: ...
) -> c_int {
    let ap = &raw mut args as *mut VaList;
    vsnprintf(s, n, fmt, ap)
}
