//! vasprintf — 动态分配缓冲区的格式化输出（va_list 版本，GNU 扩展）。
//! 对应 musl src/stdio/vasprintf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int};

/// vasprintf — 动态分配缓冲区并格式化输出。
#[no_mangle]
pub extern "C" fn vasprintf(_s: *mut *mut c_char, _fmt: *const c_char, _ap: *mut VaList) -> c_int {
    -1
}
