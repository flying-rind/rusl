//! vswscanf — 宽字符串格式化输入（va_list 版本）。
//! 对应 musl src/stdio/vswscanf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// vswscanf — 从宽字符串读取格式化输入。
#[no_mangle]
pub extern "C" fn vswscanf(
    _s: *const c_int,
    _fmt: *const c_int,
    _ap: *mut VaList,
) -> c_int {
    -1
}

/// __isoc99_vswscanf — vswscanf 的 C99 兼容弱别名。
#[no_mangle]
pub extern "C" fn __isoc99_vswscanf(
    s: *const c_int,
    fmt: *const c_int,
    ap: *mut VaList,
) -> c_int {
    vswscanf(s, fmt, ap)
}
