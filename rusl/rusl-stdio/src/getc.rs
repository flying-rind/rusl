//! getc — 从 FILE 流读取一个字符（宏的函数级回退实现）。
//! 对应 musl src/stdio/getc.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use super::stdio_impl::FILE;
use super::getc_unlocked::getc_unlocked;
use super::__uflow::__uflow;

/// getc_unlocked 的内联实现（等价于 musl 的 getc_unlocked 宏）。
#[inline]
unsafe fn __getc_unlocked_inline(f: *mut FILE) -> c_int {
    let f_ref = &mut *f;
    if f_ref.rpos != f_ref.rend {
        let c = *f_ref.rpos;
        f_ref.rpos = f_ref.rpos.add(1);
        c as c_int
    } else {
        __uflow(f)
    }
}

/// 从 FILE 流 f 中读取一个字符（加锁）。
/// [Visibility]: User — <stdio.h> 标准库函数（宏的备选函数实现）。
#[no_mangle]
pub extern "C" fn getc(f: *mut FILE) -> c_int {
    unsafe { getc_unlocked(f) }
}

/// glibc 兼容别名（弱别名 -> getc）。
/// [Visibility]: Internal — 供 libstdc++ 等传统 _IO_ 前缀代码使用。
#[no_mangle]
pub(crate) unsafe extern "C" fn _IO_getc(f: *mut FILE) -> c_int {
    unsafe { getc_unlocked(f) }
}
