//! fputs — 将 C 字符串写入 FILE 流（不追加换行符）。
//! 对应 musl src/stdio/fputs.c

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};
use crate::stdio_impl::FILE;

/// 将 C 字符串 s 写入 FILE 流 f（不包括结尾的 '\0'，不自动追加换行符）。
/// [Visibility]: User — <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn fputs(s: *const c_char, f: *mut FILE) -> c_int {
    unimplemented!()
}

/// 免锁版本：与 fputs 共享同一实现，调用者自行负责锁管理。
/// [Visibility]: User — POSIX 免锁扩展。
#[no_mangle]
pub extern "C" fn fputs_unlocked(s: *const c_char, f: *mut FILE) -> c_int {
    unimplemented!()
}
