//! fscanf — 从 FILE 流格式化输入。
//! 对应 musl src/stdio/fscanf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int};

/// 从 FILE 流格式化输入（可变参数版本）。
#[no_mangle]
pub unsafe extern "C" fn fscanf(f: *mut FILE, fmt: *const c_char, mut args: ...) -> c_int {
    let ap = &raw mut args as *mut VaList;
    super::vfscanf::vfscanf(f, fmt, ap)
}
