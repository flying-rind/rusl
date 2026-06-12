//! fclose — 关闭文件流。
//! 对应 musl src/stdio/fclose.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

extern "C" {
    fn free(ptr: *mut core::ffi::c_void);
}

/// fclose — 关闭文件流。刷新缓冲区、调用底层 close 回调、注销并释放 FILE 对象。
/// 永久流（stdin/stdout/stderr，带有 F_PERM 标志）不被释放。
#[no_mangle]
pub extern "C" fn fclose(f: *mut FILE) -> c_int {
    unsafe {
        if f.is_null() {
            return super::stdio_impl::EOF;
        }

        let f_ref = &mut *f;

        // 刷新缓冲区
        let mut r = super::fflush::fflush(f);

        // 调用关闭回调
        if let Some(close_fn) = f_ref.close {
            r |= close_fn(f);
        }

        // 永久流不释放内存
        if f_ref.flags & F_PERM != 0 {
            return r;
        }

        // 释放 FILE 内存
        free(f as *mut core::ffi::c_void);

        r
    }
}
