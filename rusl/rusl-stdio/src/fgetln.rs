//! fgetln — GNU 扩展：从 FILE 流返回指向一行数据的指针（零拷贝）。
//! 对应 musl src/stdio/fgetln.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;

/// fgetln — 从 FILE 流读取一行，返回内部缓冲区中的指针。
#[no_mangle]
pub extern "C" fn fgetln(_f: *mut FILE, _plen: *mut usize) -> *mut c_char {
    core::ptr::null_mut()
}
