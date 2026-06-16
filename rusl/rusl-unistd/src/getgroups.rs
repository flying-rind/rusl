//! getgroups — 获取调用进程的附加组列表。
//! 对应 musl src/unistd/getgroups.c
//!
//! SYS_getgroups 系统调用的薄封装。

use core::ffi::c_int;
use crate::import::do_syscall;

/// POSIX `getgroups` — 获取调用进程的附加组 ID 列表。
///
/// 若 `count == 0`，返回附加组的数量而不填充 `list`。
/// 若 `count > 0` 且足够大，将附加组 ID 写入 `list` 并返回实际数量。
/// 出错返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn getgroups(count: c_int, list: *mut u32) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_getgroups, count, list) as c_int }
}
