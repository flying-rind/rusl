//! 对应 musl src/stdio/ofl.c
//! 全局打开文件链表（open file list）管理

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 全局打开文件链表头指针
static mut ofl_head: *mut FILE = core::ptr::null_mut();

/// 保护 ofl_head 的自旋锁（简化：单线程环境设为 -1 表示无锁）
static mut ofl_lock: c_int = -1;

/// 指向 ofl_lock 的常量指针
#[no_mangle]
pub(crate) static mut __stdio_ofl_lockptr: *mut c_int =
    unsafe { core::ptr::addr_of_mut!(ofl_lock) };

/// 获取全局文件链表锁，返回链表头指针的地址
#[no_mangle]
pub(crate) unsafe extern "C" fn __ofl_lock() -> *mut *mut FILE {
    core::ptr::addr_of_mut!(ofl_head)
}

/// 释放全局文件链表锁
#[no_mangle]
pub(crate) unsafe extern "C" fn __ofl_unlock() {
    // 单线程环境无需操作
}
