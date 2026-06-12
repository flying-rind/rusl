//! 对应 musl src/stdio/putc_unlocked.c
//! 免锁 FILE 流单字符写入

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use super::__overflow::__overflow;
use core::ffi::c_int;

/// 免锁写入字符实现（musl putc_unlocked 宏逻辑）。
#[inline]
unsafe fn __putc_unlocked_impl(c: c_int, f: *mut FILE) -> c_int {
    let f_ref = &mut *f;
    let uc = c as u8;
    if uc != f_ref.lbf as u8 && f_ref.wpos != f_ref.wend {
        *f_ref.wpos = uc;
        f_ref.wpos = f_ref.wpos.add(1);
        c
    } else {
        __overflow(f, c)
    }
}

/// 将字符 c 写入 FILE 流 f，不获取流锁
#[no_mangle]
pub extern "C" fn putc_unlocked(c: c_int, f: *mut FILE) -> c_int {
    unsafe { __putc_unlocked_impl(c, f) }
}

/// fputc_unlocked — putc_unlocked 的弱别名，POSIX 标准名称
#[no_mangle]
pub extern "C" fn fputc_unlocked(c: c_int, f: *mut FILE) -> c_int {
    unsafe { __putc_unlocked_impl(c, f) }
}

/// _IO_putc_unlocked — putc_unlocked 的弱别名，glibc 兼容符号
#[no_mangle]
pub(crate) unsafe extern "C" fn _IO_putc_unlocked(c: c_int, f: *mut FILE) -> c_int {
    unsafe { __putc_unlocked_impl(c, f) }
}
