//! fprintf — 格式化输出到 FILE 流。
//! 对应 musl src/stdio/fprintf.c
//!
//! 使用 Rust `c_variadic` feature 提取 va_list, 委托给 vfprintf。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use super::vfprintf::vfprintf;
use core::ffi::{c_char, c_int};

/// 将格式化字符串输出到 FILE 流 f。
#[no_mangle]
pub unsafe extern "C" fn fprintf(f: *mut FILE, fmt: *const c_char, mut args: ...) -> c_int {
    let ap = &raw mut args as *mut VaList;
    vfprintf(f, fmt, ap)
}
