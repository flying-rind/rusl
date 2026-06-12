//! ext2 — GNU stdio_ext.h 扩展函数（第二部分）。
//! 对应 musl src/stdio/ext2.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int};

/// __freadahead — 返回读缓冲区中还可读取的字节数（rend - rpos）。
#[no_mangle]
pub extern "C" fn __freadahead(f: *mut FILE) -> usize {
    unsafe {
        let f_ref = &*f;
        if f_ref.rend > f_ref.rpos {
            (f_ref.rend as usize).wrapping_sub(f_ref.rpos as usize)
        } else {
            0
        }
    }
}

/// __freadptr — 返回指向读缓冲区当前位置的指针，并通过 *sizep 返回可读字节数。
#[no_mangle]
pub extern "C" fn __freadptr(
    f: *mut FILE,
    sizep: *mut usize,
) -> *const c_char {
    unsafe {
        let f_ref = &*f;
        if f_ref.rpos != f_ref.rend {
            *sizep = (f_ref.rend as usize).wrapping_sub(f_ref.rpos as usize);
            f_ref.rpos as *const c_char
        } else {
            core::ptr::null()
        }
    }
}

/// __freadptrinc — 将读缓冲区的读指针推进 inc 字节。
#[no_mangle]
pub extern "C" fn __freadptrinc(f: *mut FILE, inc: usize) {
    unsafe {
        let f_ref = &mut *f;
        f_ref.rpos = f_ref.rpos.add(inc);
    }
}

/// __fseterr — 直接设置 FILE 流的错误标志位（F_ERR）。
#[no_mangle]
pub extern "C" fn __fseterr(f: *mut FILE) {
    unsafe {
        let f_ref = &mut *f;
        f_ref.flags |= F_ERR;
    }
}
