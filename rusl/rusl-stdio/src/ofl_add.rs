//! 对应 musl src/stdio/ofl_add.c
//! 将新打开的 FILE 对象添加到全局打开文件链表

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;

/// 将新 FILE 对象插入全局打开文件链表头部
#[no_mangle]
pub(crate) unsafe extern "C" fn __ofl_add(f: *mut FILE) -> *mut FILE {
    unimplemented!()
}
