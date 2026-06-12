//! vfwprintf — 宽字符格式化输出核心引擎。
//! 对应 musl src/stdio/vfwprintf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// vfwprintf — 向 FILE 流写入宽字符格式化输出。
#[no_mangle]
pub extern "C" fn vfwprintf(_f: *mut FILE, _fmt: *const c_int, _ap: *mut VaList) -> c_int {
    -1
}
