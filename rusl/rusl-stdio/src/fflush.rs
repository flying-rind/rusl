//! fflush — 刷新文件流缓冲区。
//! 对应 musl src/stdio/fflush.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// fflush — 刷新 FILE 流的缓冲区（加锁版本）。
/// - 若 f 非 NULL：刷新该特定流的缓冲区
/// - 若 f 为 NULL：刷新所有当前打开的流
#[no_mangle]
pub extern "C" fn fflush(_f: *mut FILE) -> c_int {
    unimplemented!()
}

/// fflush_unlocked — fflush 的弱别名。行为与 fflush 完全一致，但不执行 FILE 对象级锁定。
#[no_mangle]
pub extern "C" fn fflush_unlocked(_f: *mut FILE) -> c_int {
    unimplemented!()
}
