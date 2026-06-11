//! tmpfile — 创建临时文件，关闭或程序退出时自动删除。
//! 对应 musl src/stdio/tmpfile.c
//!
//! 在 /tmp 下以 0600 权限原子创建文件并立即 unlink，
//! 通过返回的 *mut FILE 访问。最多 100 次重试。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// tmpfile — 创建临时文件（"w+" 模式），关闭时自动删除。
///
/// 返回值：*mut FILE 成功；null_mut() 失败并设置 errno。
#[no_mangle]
pub extern "C" fn tmpfile() -> *mut FILE {
    unimplemented!()
}
