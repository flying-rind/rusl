//! fdatasync — 同步文件数据到磁盘。
//! 对应 musl src/unistd/fdatasync.c
//!
//! SYS_fdatasync 系统调用的薄封装。

use core::ffi::c_int;
use crate::import::do_syscall;

/// fdatasync(fd) — 将文件描述符 `fd` 的已修改数据同步到磁盘。
///
/// 与 fsync 不同，不强制刷新元数据（除非元数据对后续读取必要），
/// 因此可能比 fsync 更高效。成功返回 0，出错返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn fdatasync(fd: c_int) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_fdatasync, fd) as c_int }
}
