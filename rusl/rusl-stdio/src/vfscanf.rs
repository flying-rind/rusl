//! vfscanf — 格式化输入核心引擎。
//! 对应 musl src/stdio/vfscanf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;
use core::ffi::c_int;

/// vfscanf — 从 FILE 流读取格式化输入。
#[no_mangle]
pub extern "C" fn vfscanf(_f: *mut FILE, _fmt: *const c_char, _ap: *mut VaList) -> c_int {
    -1
}

/// __isoc99_vfscanf — vfscanf 的 C99 兼容弱别名。
#[no_mangle]
pub extern "C" fn __isoc99_vfscanf(f: *mut FILE, fmt: *const c_char, ap: *mut VaList) -> c_int {
    vfscanf(f, fmt, ap)
}
