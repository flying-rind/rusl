//! fgets — 从 FILE 流中读取一行字符串到用户缓冲区。
//! 对应 musl src/stdio/fgets.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int};

/// fgets — 从 FILE 流中读取至多 n-1 个字符到 s，遇到 '\n' 或 EOF 时停止。
/// 读取的字符串以 '\0' 结尾（n >= 1 时）。换行符保留在缓冲区中。
/// 返回 s（成功）或 NULL（失败/EOF 且未读取任何字符）。
#[no_mangle]
pub extern "C" fn fgets(
    _s: *mut c_char,
    _n: c_int,
    _f: *mut FILE,
) -> *mut c_char {
    unimplemented!()
}

/// fgets_unlocked — fgets 的弱别名。行为与 fgets 完全一致。
#[no_mangle]
pub extern "C" fn fgets_unlocked(
    _s: *mut c_char,
    _n: c_int,
    _f: *mut FILE,
) -> *mut c_char {
    unimplemented!()
}
