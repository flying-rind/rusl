//! linkat — 相对于目录文件描述符创建硬链接。
//! 对应 musl src/unistd/linkat.c
//!
//! SYS_linkat 系统调用的薄封装。

use core::ffi::{c_char, c_int};
use rusl_internal::do_syscall;

/// linkat(fd1, existing, fd2, new, flag) — 相对于目录 fd 创建硬链接。
///
/// - fd1: `existing` 的基目录 fd 或 AT_FDCWD
/// - fd2: `new` 的基目录 fd 或 AT_FDCWD
/// - flag: 0 或 AT_SYMLINK_FOLLOW
///
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn linkat(
    fd1: c_int,
    existing: *const c_char,
    fd2: c_int,
    new: *const c_char,
    flag: c_int,
) -> c_int {
    unsafe { do_syscall!(rusl_internal::syscall::SYS_linkat, fd1, existing, fd2, new, flag) as c_int }
}
