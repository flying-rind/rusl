//! getw — 从 FILE 流中读取一个 int 整数的二进制表示。
//! 对应 musl src/stdio/getw.c
//! SVID 兼容 / GNU 扩展函数（需 _GNU_SOURCE）。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use crate::stdio_impl::FILE;

/// 从 FILE 流 f 中读取 sizeof(int) 字节的二进制数据并解释为 int。
/// 失败返回 EOF（-1）。调用者需使用 feof(f)/ferror(f) 区分真实 EOF 和读到值 -1。
/// [Visibility]: User — SVID/GNU 扩展函数（需 _GNU_SOURCE）。
#[no_mangle]
pub extern "C" fn getw(f: *mut FILE) -> c_int {
    unimplemented!()
}
