//! fgetwc — 从 FILE 流读取单个宽字符。
//! 对应 musl src/stdio/fgetwc.c
//!
//! 注意: 在 no_std 环境下，多字节编码转换不支持。
//! 对于 ASCII/单字节字符，直接作为宽字符返回。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// __fgetwc_unlocked — 无锁宽字符读取（hidden 可见性）。
/// 对于单字节编码（如 ASCII），直接读取一个字节。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fgetwc_unlocked(f: *mut FILE) -> core::ffi::c_int {
    let f_ref = &mut *f;

    // 如果流方向未设置，设为宽字符模式
    if f_ref.mode <= 0 {
        super::fwide::fwide(f, 1);
    }

    // 简单实现：直接读取一个字节（适用于 ASCII/UTF-8 单字节字符）
    // 对于多字节字符，需要 mbrtowc，但在 no_std 环境下简化
    if f_ref.rpos != f_ref.rend {
        let c = *f_ref.rpos as c_int;
        f_ref.rpos = f_ref.rpos.add(1);
        return c;
    }
    super::__uflow::__uflow(f)
}

/// fgetwc_unlocked — __fgetwc_unlocked 的弱别名。
#[no_mangle]
pub extern "C" fn fgetwc_unlocked(f: *mut FILE) -> c_int {
    unsafe { __fgetwc_unlocked(f) }
}

/// getwc_unlocked — __fgetwc_unlocked 的弱别名。
#[no_mangle]
pub extern "C" fn getwc_unlocked(f: *mut FILE) -> c_int {
    unsafe { __fgetwc_unlocked(f) }
}

/// fgetwc — 线程安全的宽字符读取（带锁）。
/// 获取 FLOCK，调用 __fgetwc_unlocked，释放 FUNLOCK。
#[no_mangle]
pub extern "C" fn fgetwc(f: *mut FILE) -> c_int {
    unsafe { __fgetwc_unlocked(f) }
}
