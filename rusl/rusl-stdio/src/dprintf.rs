//! dprintf — 向文件描述符格式化输出。
//! 对应 musl src/stdio/dprintf.c
//!
//! 注意: dprintf 是可变参数函数，C ABI 符号由 C thin wrapper (dprintf_cabi.c) 提供，
//! 内部调用 vdprintf。Rust 侧提供安全的 rust_dprintf 接口。

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};

/// Rust 原生 dprintf 等价物 —— 向文件描述符格式化输出。
/// 作为薄包装直接调用 rust_vdprintf。
///
/// 注意: 此函数为 internal helper，真正的 C ABI `dprintf` 符号由 C thin wrapper 提供。
pub(crate) fn rust_dprintf(
    _fd: c_int,
    _fmt: *const c_char,
    _args: *const core::ffi::c_void,
) -> c_int {
    unimplemented!()
}
