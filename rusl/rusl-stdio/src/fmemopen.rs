//! fmemopen — 创建内存流 FILE 对象。
//! 对应 musl src/stdio/fmemopen.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int, c_void};

/// fmemopen — 创建一个将内存缓冲区作为文件进行读写操作的 FILE 流。
/// - buf: 用户提供的缓冲区指针（可为 NULL，此时内部分配）
/// - size: 缓冲区大小
/// - mode: 模式字符串，首字符 'r'/'w'/'a'，可选含 '+' 表示可读写
/// 返回新创建的 FILE 指针，失败返回 NULL。
#[no_mangle]
pub extern "C" fn fmemopen(
    _buf: *mut c_void,
    _size: usize,
    _mode: *const c_char,
) -> *mut FILE {
    unimplemented!()
}
