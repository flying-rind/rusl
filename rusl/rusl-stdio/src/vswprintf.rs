//! vswprintf — 宽字符串格式化输出（va_list 版本）。
//! 对应 musl src/stdio/vswprintf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// vswprintf — 将格式化宽字符串写入缓冲区 s，最多 n 个宽字符。
#[no_mangle]
pub extern "C" fn vswprintf(
    _s: *mut c_int,
    _n: usize,
    _fmt: *const c_int,
    _ap: *mut VaList,
) -> c_int {
    -1
}
