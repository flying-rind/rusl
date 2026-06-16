//! dup2 — 复制文件描述符到指定编号。
//! 对应 musl src/unistd/dup2.c
//!
//! SYS_dup2 系统调用的薄封装。

use core::ffi::c_int;
use crate::import::do_syscall;

/// dup2(old, new) — 复制 `old` 到 `new`，若 `new` 已打开则先关闭。
///
/// 使用原子性的 dup2 系统调用避免竞态条件。
/// `old == new` 时仅检查 old 有效性后返回 new。
/// 成功返回 new，出错返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn dup2(old: c_int, new: c_int) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_dup2, old, new) as c_int }
}
