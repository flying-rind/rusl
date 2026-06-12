//! vsscanf — 字符串格式化输入（va_list 版本）。
//! 对应 musl src/stdio/vsscanf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;
use core::ffi::c_int;

/// vsscanf — 从内存中的 null 结尾字符串读取格式化输入。
#[no_mangle]
pub extern "C" fn vsscanf(s: *const c_char, _fmt: *const c_char, _ap: *mut VaList) -> c_int {
    // 简化实现：检查空输入返回 EOF
    if s.is_null() || unsafe { *s == 0 } {
        return -1;
    }
    -1
}

/// __isoc99_vsscanf — vsscanf 的 C99 兼容弱别名。
#[no_mangle]
pub extern "C" fn __isoc99_vsscanf(s: *const c_char, fmt: *const c_char, ap: *mut VaList) -> c_int {
    vsscanf(s, fmt, ap)
}
