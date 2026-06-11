//! ext2 — GNU stdio_ext.h 扩展函数（第二部分）。
//! 对应 musl src/stdio/ext2.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int};

/// __freadahead — 返回读缓冲区中还可读取的字节数（rend - rpos）。
#[no_mangle]
pub extern "C" fn __freadahead(_f: *mut FILE) -> usize {
    unimplemented!()
}

/// __freadptr — 返回指向读缓冲区当前位置的指针，并通过 *sizep 返回可读字节数。
/// 若缓冲区为空（rpos == rend），返回 NULL 且不修改 *sizep。
#[no_mangle]
pub extern "C" fn __freadptr(
    _f: *mut FILE,
    _sizep: *mut usize,
) -> *const c_char {
    unimplemented!()
}

/// __freadptrinc — 将读缓冲区的读指针推进 inc 字节。
/// 与 __freadptr 配合实现零拷贝读取。
#[no_mangle]
pub extern "C" fn __freadptrinc(_f: *mut FILE, _inc: usize) {
    unimplemented!()
}

/// __fseterr — 直接设置 FILE 流的错误标志位（F_ERR）。
/// 手动将流标记为错误状态。
#[no_mangle]
pub extern "C" fn __fseterr(_f: *mut FILE) {
    unimplemented!()
}
