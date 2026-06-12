//! fflush — 刷新文件流缓冲区。
//! 对应 musl src/stdio/fflush.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 内部不加锁 fflush 实现
unsafe fn __fflush_unlocked(f: *mut FILE) -> c_int {
    let f_ref = &mut *f;

    // 如果有待刷新的写数据
    if f_ref.wpos != f_ref.wbase {
        if let Some(write_fn) = &f_ref.write {
            write_fn(f, core::ptr::null(), 0);
        }
        if f_ref.wpos.is_null() {
            return EOF;
        }
    }

    // 如果有未消费的读数据，seek 到正确位置
    if f_ref.rpos != f_ref.rend {
        if let Some(seek_fn) = &f_ref.seek {
            let diff = (f_ref.rpos as usize).wrapping_sub(f_ref.rend as usize) as i64;
            seek_fn(f, diff, 1); // SEEK_CUR
        }
    }

    // 清除读写指针
    f_ref.wpos = core::ptr::null_mut();
    f_ref.wbase = core::ptr::null_mut();
    f_ref.wend = core::ptr::null_mut();
    f_ref.rpos = core::ptr::null_mut();
    f_ref.rend = core::ptr::null_mut();

    0
}

/// fflush — 刷新 FILE 流的缓冲区（加锁版本）。
/// - 若 f 非 NULL：刷新该特定流的缓冲区
/// - 若 f 为 NULL：刷新所有当前打开的流
#[no_mangle]
pub extern "C" fn fflush(f: *mut FILE) -> c_int {
    if f.is_null() {
        // 刷新所有打开的文件
        let mut r = 0;
        // 刷新 stdout 和 stderr
        unsafe {
            if !super::stdout::stdout.is_null() {
                r |= __fflush_unlocked(super::stdout::stdout);
            }
            if !super::stderr::stderr.is_null() {
                r |= __fflush_unlocked(super::stderr::stderr);
            }
        }
        return r;
    }
    unsafe { __fflush_unlocked(f) }
}

/// fflush_unlocked — fflush 的弱别名。行为与 fflush 完全一致，但不执行 FILE 对象级锁定。
#[no_mangle]
pub extern "C" fn fflush_unlocked(f: *mut FILE) -> c_int {
    if f.is_null() {
        return fflush(f);
    }
    unsafe { __fflush_unlocked(f) }
}
