//! getc_unlocked — 免锁从 FILE 流读取一个字符。
//! 对应 musl src/stdio/getc_unlocked.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use crate::stdio_impl::FILE;

/// 从 FILE 流 f 中读取一个字符（不加锁）。调用者负责锁管理。
/// [Visibility]: User — POSIX 免锁扩展。
#[no_mangle]
pub extern "C" fn getc_unlocked(f: *mut FILE) -> c_int {
    unimplemented!()
}

/// POSIX 标准名称（弱别名 -> getc_unlocked）。
/// [Visibility]: User — POSIX 免锁扩展。
#[no_mangle]
pub extern "C" fn fgetc_unlocked(f: *mut FILE) -> c_int {
    unimplemented!()
}

/// glibc 兼容别名（弱别名 -> getc_unlocked）。
/// [Visibility]: Internal — 供要求 _IO_* 符号的旧代码使用。
#[no_mangle]
pub(crate) unsafe extern "C" fn _IO_getc_unlocked(f: *mut FILE) -> c_int {
    unimplemented!()
}
