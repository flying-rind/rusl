//! __lockfile / __unlockfile — FILE 对象的原子 CAS + futex 递归锁。
//! 对应 musl src/stdio/__lockfile.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use super::stdio_impl::FILE;

/// MAYBE_WAITERS 标志位（musl 定义: 0x40000000）
pub const MAYBE_WAITERS: c_int = 0x40000000;

/// 获取 FILE 流 f 的递归锁。
/// 在单线程 / lock==-1 时直接返回 1。
/// [Visibility]: Internal (hidden) — 由 FLOCK() 宏在 stdio 函数入口处调用。
#[no_mangle]
pub(crate) unsafe extern "C" fn __lockfile(f: *mut FILE) -> c_int {
    let f_ref = &*f;
    if f_ref.lock < 0 {
        return 1;
    }
    // 简化：单线程环境下，直接返回 1
    // 多线程实现在 no_std 环境下暂不支持
    1
}

/// 释放 FILE 流 f 的递归锁。
/// [Visibility]: Internal (hidden) — 由 FUNLOCK() 宏在 stdio 函数出口处调用。
#[no_mangle]
pub(crate) unsafe extern "C" fn __unlockfile(_f: *mut FILE) {
    // 简化：单线程环境下无需释放
}
