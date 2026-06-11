//! __lockfile / __unlockfile — FILE 对象的原子 CAS + futex 递归锁。
//! 对应 musl src/stdio/__lockfile.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use crate::stdio_impl::FILE;

/// MAYBE_WAITERS 标志位（musl 定义: 0x40000000）
pub const MAYBE_WAITERS: c_int = 0x40000000;

/// 获取 FILE 流 f 的递归锁。
/// 返回 1: 首次获锁（调用方需在退出时调用 __unlockfile）。
/// 返回 0: 递归获锁（同一线程已持锁，调用方不应调用 __unlockfile）。
/// [Visibility]: Internal (hidden) — 由 FLOCK() 宏在 stdio 函数入口处调用。
#[no_mangle]
pub(crate) unsafe extern "C" fn __lockfile(f: *mut FILE) -> c_int {
    unimplemented!()
}

/// 释放 FILE 流 f 的递归锁，必要时通过 futex_wake 唤醒等待者。
/// [Visibility]: Internal (hidden) — 由 FUNLOCK() 宏在 stdio 函数出口处调用。
#[no_mangle]
pub(crate) unsafe extern "C" fn __unlockfile(f: *mut FILE) {
    unimplemented!()
}
