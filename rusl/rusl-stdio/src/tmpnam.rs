//! tmpnam — 临时文件名生成（C89，已过时）。
//! 对应 musl src/stdio/tmpnam.c
//!
//! 生成格式为 /tmp/tmpnam_XXXXXX 的唯一路径名，不创建文件。
//! 最多 100 次重试。
//!
//! 安全警告：存在 TOCTOU 竞态条件，非线程安全（内部静态缓冲区），POSIX 已标记过时。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;

/// tmpnam — 生成唯一临时文件名。
///
/// - `buf`: 缓冲区（至少 L_tmpnam 字节），NULL 时使用内部静态缓冲区
///
/// 返回值：指向生成路径的指针（buf 或内部静态缓冲区）；NULL 表示失败。
#[no_mangle]
pub extern "C" fn tmpnam(buf: *mut c_char) -> *mut c_char {
    unimplemented!()
}
