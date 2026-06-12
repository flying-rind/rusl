//! ext — GNU stdio_ext.h 扩展函数（第一部分）。
//! 对应 musl src/stdio/ext.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// _flushlbf — 刷新所有行缓冲的 FILE 流。
#[no_mangle]
pub extern "C" fn _flushlbf() {
    super::fflush::fflush(core::ptr::null_mut());
}

/// __fsetlocking — 设置 FILE 流的锁定行为。
#[no_mangle]
pub extern "C" fn __fsetlocking(_f: *mut FILE, _type_: c_int) -> c_int {
    0
}

/// __fwriting — 查询流是否处于"正在写入"状态。
#[no_mangle]
pub extern "C" fn __fwriting(f: *mut FILE) -> c_int {
    unsafe { let f_ref = &*f; if f_ref.flags & F_NORD != 0 || f_ref.wpos != f_ref.wbase { 1 } else { 0 } }
}

/// __freading — 查询流是否处于"正在读取"状态。
#[no_mangle]
pub extern "C" fn __freading(f: *mut FILE) -> c_int {
    unsafe { let f_ref = &*f; if f_ref.flags & F_NOWR != 0 || f_ref.rpos != f_ref.rend { 1 } else { 0 } }
}

/// __freadable — 查询流是否可读（F_NORD 未设置）。
#[no_mangle]
pub extern "C" fn __freadable(f: *mut FILE) -> c_int {
    unsafe { if (*f).flags & F_NORD == 0 { 1 } else { 0 } }
}

/// __fwritable — 查询流是否可写（F_NOWR 未设置）。
#[no_mangle]
pub extern "C" fn __fwritable(f: *mut FILE) -> c_int {
    unsafe { if (*f).flags & F_NOWR == 0 { 1 } else { 0 } }
}

/// __flbf — 查询流是否使用行缓冲模式（lbf >= 0）。
#[no_mangle]
pub extern "C" fn __flbf(f: *mut FILE) -> c_int {
    unsafe { if (*f).lbf >= 0 { 1 } else { 0 } }
}

/// __fbufsize — 返回流的缓冲区大小（buf_size 字段）。
#[no_mangle]
pub extern "C" fn __fbufsize(f: *mut FILE) -> usize {
    unsafe { (*f).buf_size }
}

/// __fpending — 返回写缓冲区中待写入的字节数（wpos - wbase）。
#[no_mangle]
pub extern "C" fn __fpending(f: *mut FILE) -> usize {
    unsafe {
        let f_ref = &*f;
        if f_ref.wpos > f_ref.wbase {
            (f_ref.wpos as usize).wrapping_sub(f_ref.wbase as usize)
        } else {
            0
        }
    }
}

/// __fpurge — 清空 FILE 流的所有内部缓冲区。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fpurge(f: *mut FILE) -> c_int {
    unsafe {
        let f_ref = &mut *f;
        f_ref.wpos = core::ptr::null_mut();
        f_ref.wbase = core::ptr::null_mut();
        f_ref.wend = core::ptr::null_mut();
        f_ref.rpos = core::ptr::null_mut();
        f_ref.rend = core::ptr::null_mut();
        0
    }
}

/// fpurge — __fpurge 的弱别名。
#[no_mangle]
pub extern "C" fn fpurge(f: *mut FILE) -> c_int {
    unsafe { __fpurge(f) }
}
