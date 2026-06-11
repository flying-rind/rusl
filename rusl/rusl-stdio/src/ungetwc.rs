//! ungetwc — 将宽字符推回 FILE 流的输入缓冲区。
//! 对应 musl src/stdio/ungetwc.c
//!
//! 需要处理多字节编码转换和 locale 管理。无论成功或失败，
//! 调用前后的 locale 值必须一致（locale 安全性）。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_int, c_uint};

/// wint_t 在 musl 中定义为 unsigned (c_uint)
pub type wint_t = c_uint;

/// ungetwc — 将宽字符 c 推回流 f 的读缓冲区。
///
/// - `c`: 要推回的宽字符（wint_t），WEOF 不可推回
/// - `f`: 目标 FILE 流
///
/// 返回值：成功时返回 c；失败时返回 WEOF。
/// 非 ASCII 字符通过 wcrtomb 转换为多字节序列后推回。
#[no_mangle]
pub extern "C" fn ungetwc(c: wint_t, f: *mut FILE) -> wint_t {
    unimplemented!()
}
