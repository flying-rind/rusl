//! tempnam — 可定制临时文件名生成（POSIX XSI 扩展，已过时）。
//! 对应 musl src/stdio/tempnam.c
//!
//! 生成格式为 <dir>/<pfx>_XXXXXX 的唯一路径名，不创建文件。
//! 返回值由内部 malloc 分配，调用者负责 free。
//!
//! 安全警告：存在 TOCTOU 竞态条件，被 POSIX 标记为过时。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;

/// tempnam — 生成唯一临时文件名。
///
/// - `dir`: 目录，NULL 时使用 P_tmpdir（"/tmp"）
/// - `pfx`: 前缀，NULL 时使用 "temp"
///
/// 返回值：malloc 分配的路径名字符串，调用者负责 free；NULL 表示失败。
#[no_mangle]
pub extern "C" fn tempnam(dir: *const c_char, pfx: *const c_char) -> *mut c_char {
    unimplemented!()
}
