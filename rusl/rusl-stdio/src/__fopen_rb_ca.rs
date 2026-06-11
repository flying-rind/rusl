//! __fopen_rb_ca — 调用方分配 FILE（Caller-Allocated）的只读打开实现。
//! 对应 musl src/stdio/__fopen_rb_ca.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
use crate::stdio_impl::FILE;

/// 内部函数：以只读方式打开文件，使用调用方提供的 FILE 内存和缓冲区。
/// [Visibility]: Internal (hidden) — 由 freopen 等内部调用。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fopen_rb_ca(
    filename: *const c_char,
    f: *mut FILE,
    buf: *mut u8,
    len: usize,
) -> *mut FILE {
    unimplemented!()
}
