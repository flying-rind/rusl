//! fseek / fseeko — 文件流定位操作。
//! 对应 musl src/stdio/fseek.c

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_int, c_long};
use crate::stdio_impl::FILE;

/// off_t 类型（x86_64 上为 c_long = i64）
pub type off_t = c_long;

/// 内部不加锁文件定位引擎。
/// [Visibility]: Internal (hidden) — 由 __fseeko / __fseeko_unlocked 内部调用。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fseeko_unlocked(f: *mut FILE, off: off_t, whence: c_int) -> c_int {
    unimplemented!()
}

/// 内部加锁文件定位（fseeko 的主实现）。
/// [Visibility]: Internal (hidden) — 由 fseek / fsetpos 等调用。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fseeko(f: *mut FILE, off: off_t, whence: c_int) -> c_int {
    unimplemented!()
}

/// 标准文件定位（c_long 偏移量）。
/// [Visibility]: User — ISO C / POSIX <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn fseek(f: *mut FILE, off: c_long, whence: c_int) -> c_int {
    unimplemented!()
}

/// POSIX 大文件定位（off_t 偏移量，弱别名 -> __fseeko）。
/// [Visibility]: User — POSIX 标准函数（需 _POSIX_C_SOURCE >= 200112L）。
#[no_mangle]
pub extern "C" fn fseeko(f: *mut FILE, off: off_t, whence: c_int) -> c_int {
    unimplemented!()
}
