//! fsync — 同步文件数据和元数据到磁盘。
//! 对应 musl src/unistd/fsync.c
//!
//! SYS_fsync 系统调用的薄封装。

use core::ffi::c_int;
use crate::import::do_syscall;

/// fsync(fd) — 将文件描述符 `fd` 的所有已修改数据和元数据同步到磁盘。
///
/// 确保系统崩溃后数据不丢失。成功返回 0，出错返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn fsync(fd: c_int) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_fsync, fd) as c_int }
}
