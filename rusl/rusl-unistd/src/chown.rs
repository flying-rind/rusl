//! chown — 改变文件的所有者和/或所属组。
//! 对应 musl src/unistd/chown.c
//!
//! SYS_chown 系统调用的薄封装。

use core::ffi::{c_char, c_int};
use crate::import::do_syscall;

/// chown(path, uid, gid) — 将文件 `path` 的所有者/组改为 `uid`/`gid`。
///
/// uid 或 gid 为 -1 时，对应属性保持不变。
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn chown(path: *const c_char, uid: u32, gid: u32) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_chown, path, uid, gid) as c_int }
}
