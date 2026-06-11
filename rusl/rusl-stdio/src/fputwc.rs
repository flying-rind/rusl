//! fputwc / putwc — 宽字符单字符写入。
//! 对应 musl src/stdio/fputwc.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_uint;
use crate::stdio_impl::FILE;

/// 内部不加锁宽字符写入引擎。
/// [Visibility]: Internal (hidden) — 由 fputwc / fputwc_unlocked / putwc_unlocked 调用。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fputwc_unlocked(c: c_uint /* wchar_t */, f: *mut FILE) -> c_uint /* wint_t */ {
    unimplemented!()
}

/// 加锁宽字符写入。
/// [Visibility]: User — <wchar.h> 标准库函数。
#[no_mangle]
pub extern "C" fn fputwc(c: c_uint /* wchar_t */, f: *mut FILE) -> c_uint /* wint_t */ {
    unimplemented!()
}

/// 免锁宽字符写入（弱别名 -> __fputwc_unlocked）。
/// [Visibility]: User — POSIX 免锁扩展。
#[no_mangle]
pub extern "C" fn fputwc_unlocked(c: c_uint /* wchar_t */, f: *mut FILE) -> c_uint /* wint_t */ {
    unimplemented!()
}

/// 免锁宽字符写入（弱别名 -> __fputwc_unlocked）。
/// [Visibility]: User — POSIX 免锁扩展。
#[no_mangle]
pub extern "C" fn putwc_unlocked(c: c_uint /* wchar_t */, f: *mut FILE) -> c_uint /* wint_t */ {
    unimplemented!()
}
