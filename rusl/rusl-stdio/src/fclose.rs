//! fclose — 关闭文件流。
//! 对应 musl src/stdio/fclose.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// fclose — 关闭文件流。刷新缓冲区、调用底层 close 回调、注销并释放 FILE 对象。
/// 永久流（stdin/stdout/stderr，带有 F_PERM 标志）不被释放。
#[no_mangle]
pub extern "C" fn fclose(_f: *mut FILE) -> c_int {
    unimplemented!()
}
