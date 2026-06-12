//! fputs — 将 C 字符串写入 FILE 流（不追加换行符）。
//! 对应 musl src/stdio/fputs.c

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};
use super::stdio_impl::FILE;

/// 内部实现：将 C 字符串 s 写入 FILE 流 f
unsafe fn __fputs_impl(s: *const c_char, f: *mut FILE) -> c_int {
    let len = crate::import::strnlen(s, usize::MAX);
    let n = super::fwrite::__fwritex(s as *const u8, len, f);
    if n < len { super::stdio_impl::EOF } else { 0 }
}

/// 将 C 字符串 s 写入 FILE 流 f（不包括结尾的 '\0'，不自动追加换行符）。
/// [Visibility]: User — <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn fputs(s: *const c_char, f: *mut FILE) -> c_int {
    unsafe { __fputs_impl(s, f) }
}

/// 免锁版本：与 fputs 共享同一实现，调用者自行负责锁管理。
/// [Visibility]: User — POSIX 免锁扩展。
#[no_mangle]
pub extern "C" fn fputs_unlocked(s: *const c_char, f: *mut FILE) -> c_int {
    unsafe { __fputs_impl(s, f) }
}
