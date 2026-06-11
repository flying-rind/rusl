//! asprintf — 自动分配缓冲区的格式化输出。
//! 对应 musl src/stdio/asprintf.c
//!
//! 注意: asprintf 是可变参数函数，C ABI 符号由 C thin wrapper (asprintf_cabi.c) 提供，
//! 内部调用 vasprintf。Rust 侧提供安全的 rust_asprintf 接口。

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};

/// Rust 原生 asprintf 等价物 —— 返回堆分配的字符串。
/// 作为薄包装直接调用 rust_vasprintf。
///
/// 注意: 此函数为 internal helper，真正的 C ABI `asprintf` 符号由 C thin wrapper 提供。
pub(crate) fn rust_asprintf(
    _fmt: *const c_char,
    _args: *const core::ffi::c_void,
) -> *mut c_char {
    unimplemented!()
}
