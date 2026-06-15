//! fchownat — 相对于目录文件描述符改变文件的所有者和/或所属组。
//! 对应 musl src/unistd/fchownat.c
//!
//! SYS_fchownat 系统调用的薄封装。

use core::ffi::{c_char, c_int};
use rusl_internal::do_syscall;

/// fchownat(fd, path, uid, gid, flag) — 相对于目录 `fd` 改变文件所有者/组。
///
/// - fd: 目录文件描述符或 AT_FDCWD (-100)
/// - flag: 0 或 AT_SYMLINK_NOFOLLOW (0x100)
/// - uid/gid 为 -1 时保持不变
///
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn fchownat(
    fd: c_int,
    path: *const c_char,
    uid: u32,
    gid: u32,
    flag: c_int,
) -> c_int {
    unsafe { do_syscall!(rusl_internal::syscall::SYS_fchownat, fd, path, uid, gid, flag) as c_int }
}
