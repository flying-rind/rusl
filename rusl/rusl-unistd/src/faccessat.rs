//! faccessat — 相对于目录文件描述符检查文件访问权限。
//! 对应 musl src/unistd/faccessat.c
//!
//! SYS_faccessat 系统调用的薄封装。

use core::ffi::{c_char, c_int};
use rusl_internal::do_syscall;

/// faccessat(fd, filename, amode, flag) — 相对于目录 `fd` 检查文件访问权限。
///
/// - fd: 目录文件描述符或 AT_FDCWD (-100)
/// - amode: F_OK、R_OK|W_OK|X_OK 的组合
/// - flag: 0、AT_EACCESS 或 AT_SYMLINK_NOFOLLOW 的组合
///
/// 允许访问返回 0，拒绝返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn faccessat(
    fd: c_int,
    filename: *const c_char,
    amode: c_int,
    flag: c_int,
) -> c_int {
    unsafe { do_syscall!(rusl_internal::syscall::SYS_faccessat, fd, filename, amode, flag) as c_int }
}
