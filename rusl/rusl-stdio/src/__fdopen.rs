//! __fdopen — 从已打开的文件描述符构造 FILE 流对象。
//! 对应 musl src/stdio/__fdopen.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int};

/// __fdopen — 主实现。从 fd 和 mode 字符串构造 FILE，分配内存、配置缓冲区、设置操作指针。
/// 分配 sizeof(FILE) + UNGET + BUFSIZ 字节，将流登记到全局打开文件链表。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fdopen(
    _fd: c_int,
    _mode: *const c_char,
) -> *mut FILE {
    unimplemented!()
}

/// fdopen — __fdopen 的弱别名。对外导出，行为与 __fdopen 完全一致。
#[no_mangle]
pub(crate) unsafe extern "C" fn fdopen(_fd: c_int, _mode: *const c_char) -> *mut FILE {
    unimplemented!()
}
