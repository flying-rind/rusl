//! ftell / ftello — 文件流当前位置查询。
//! 对应 musl src/stdio/ftell.c

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_int, c_long};
use crate::stdio_impl::FILE;

/// off_t 类型（x86_64 上为 c_long = i64）
pub type off_t = c_long;

/// 内部不加锁位置查询引擎。
/// [Visibility]: Internal (hidden) — 由 __ftello 调用。
#[no_mangle]
pub(crate) unsafe extern "C" fn __ftello_unlocked(f: *mut FILE) -> off_t {
    unimplemented!()
}

/// 内部加锁位置查询（ftello 的主实现）。
/// [Visibility]: Internal (hidden) — 由 ftell 调用。
#[no_mangle]
pub(crate) unsafe extern "C" fn __ftello(f: *mut FILE) -> off_t {
    unimplemented!()
}

/// 标准当前位置查询（c_long 返回值，超出 LONG_MAX 时设置 EOVERFLOW）。
/// [Visibility]: User — ISO C / POSIX <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn ftell(f: *mut FILE) -> c_long {
    unimplemented!()
}

/// POSIX 大文件位置查询（off_t 返回值，弱别名 -> __ftello）。
/// [Visibility]: User — POSIX 标准函数（需 _POSIX_C_SOURCE >= 200112L）。
#[no_mangle]
pub extern "C" fn ftello(f: *mut FILE) -> off_t {
    unimplemented!()
}
