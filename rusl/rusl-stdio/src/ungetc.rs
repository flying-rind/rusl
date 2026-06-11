//! ungetc — 将单字节字符推回 FILE 流的输入缓冲区。
//! 对应 musl src/stdio/ungetc.c
//!
//! 保证至少可成功推回一个字符。成功推回后流的 EOF 状态被清除。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// ungetc — 将字符 c 推回流 f 的读缓冲区。
///
/// - `c`: 要推回的字符（c_int，仅低 8 位有效），EOF 不可推回
/// - `f`: 目标 FILE 流
///
/// 返回值：成功时返回 (c as u8 as c_int)；失败时返回 EOF。
#[no_mangle]
pub extern "C" fn ungetc(c: c_int, f: *mut FILE) -> c_int {
    unimplemented!()
}
