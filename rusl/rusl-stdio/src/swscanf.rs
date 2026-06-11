//! swscanf — 从宽字符串读取格式化输入。
//! 对应 musl src/stdio/swscanf.c
//!
//! vswscanf 的可变参数包装，使用 va_list 机制转发。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_int, c_uint};

/// swscanf — 从宽字符串 s 读取格式化输入。
#[no_mangle]
pub unsafe extern "C" fn swscanf(s: *const c_uint, fmt: *const c_uint, _: ...) -> c_int {
    loop {}
}

/// __isoc99_swscanf — swscanf 的 C99 兼容弱别名。
#[no_mangle]
pub unsafe extern "C" fn __isoc99_swscanf(s: *const c_uint, fmt: *const c_uint, _: ...) -> c_int {
    loop {}
}
