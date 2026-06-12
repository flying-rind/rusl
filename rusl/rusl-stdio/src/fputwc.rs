//! fputwc / putwc — 宽字符单字符写入。
//! 对应 musl src/stdio/fputwc.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::FILE;
use core::ffi::c_int;

/// 内部不加锁宽字符写入引擎。
/// [Visibility]: Internal (hidden) — 由 fputwc / fputwc_unlocked / putwc_unlocked 调用。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fputwc_unlocked(c: c_int /* wchar_t */, f: *mut FILE) -> c_int /* wint_t */ {
    let f_ref = &mut *f;

    // 如果流方向未设置，设为宽字符模式
    if f_ref.mode <= 0 {
        super::fwide::fwide(f, 1);
    }

    // 简单实现：直接写入一个字节（适用于 ASCII）
    if (c as u32) <= 0x7F {
        // ASCII 字符，直接写入
        return super::putc_unlocked::putc_unlocked(c, f);
    }

    // 非 ASCII 宽字符，在 no_std 环境下返回 WEOF
    f_ref.flags |= super::stdio_impl::F_ERR;
    -1
}

/// 加锁宽字符写入。
/// [Visibility]: User — <wchar.h> 标准库函数。
#[no_mangle]
pub extern "C" fn fputwc(c: c_int /* wchar_t */, f: *mut FILE) -> c_int /* wint_t */ {
    unsafe { __fputwc_unlocked(c, f) }
}

/// 免锁宽字符写入（弱别名 -> __fputwc_unlocked）。
/// [Visibility]: User — POSIX 免锁扩展。
#[no_mangle]
pub extern "C" fn fputwc_unlocked(c: c_int /* wchar_t */, f: *mut FILE) -> c_int /* wint_t */ {
    unsafe { __fputwc_unlocked(c, f) }
}

/// 免锁宽字符写入（弱别名 -> __fputwc_unlocked）。
/// [Visibility]: User — POSIX 免锁扩展。
#[no_mangle]
pub extern "C" fn putwc_unlocked(c: c_int /* wchar_t */, f: *mut FILE) -> c_int /* wint_t */ {
    unsafe { __fputwc_unlocked(c, f) }
}
