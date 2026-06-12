//! getc_unlocked — 免锁从 FILE 流读取一个字符。
//! 对应 musl src/stdio/getc_unlocked.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use super::__uflow::__uflow;
use core::ffi::c_int;

/// 免锁从 FILE 流读取一个字符（等价于 musl getc_unlocked 宏）。
#[inline]
unsafe fn __getc_unlocked_impl(f: *mut FILE) -> c_int {
    let f_ref = &mut *f;
    if f_ref.rpos != f_ref.rend {
        let c = *f_ref.rpos;
        f_ref.rpos = f_ref.rpos.add(1);
        c as c_int
    } else {
        __uflow(f)
    }
}

/// 从 FILE 流 f 中读取一个字符（不加锁）。调用者负责锁管理。
/// [Visibility]: User — POSIX 免锁扩展。
#[no_mangle]
pub extern "C" fn getc_unlocked(f: *mut FILE) -> c_int {
    unsafe { __getc_unlocked_impl(f) }
}

/// POSIX 标准名称（弱别名 -> getc_unlocked）。
/// [Visibility]: User — POSIX 免锁扩展。
#[no_mangle]
pub extern "C" fn fgetc_unlocked(f: *mut FILE) -> c_int {
    unsafe { __getc_unlocked_impl(f) }
}

/// glibc 兼容别名（弱别名 -> getc_unlocked）。
/// [Visibility]: Internal — 供要求 _IO_* 符号的旧代码使用。
#[no_mangle]
pub(crate) unsafe extern "C" fn _IO_getc_unlocked(f: *mut FILE) -> c_int {
    unsafe { __getc_unlocked_impl(f) }
}
