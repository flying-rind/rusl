//! fread — 从 FILE 流中读取指定数量的元素到用户缓冲区。
//! 对应 musl src/stdio/fread.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
use crate::stdio_impl::FILE;

/// 从 FILE 流 f 中读取 nmemb 个大小为 size 字节的元素到 destv 缓冲区。
/// [Visibility]: User — <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn fread(destv: *mut c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize {
    unimplemented!()
}

/// 免锁版本（弱别名 -> fread）。
/// [Visibility]: User — POSIX 免锁扩展。
#[no_mangle]
pub extern "C" fn fread_unlocked(destv: *mut c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize {
    unimplemented!()
}
