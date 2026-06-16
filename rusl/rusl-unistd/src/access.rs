//! access — 使用调用进程的真实 UID/GID 检查文件访问权限。
//! 对应 musl src/unistd/access.c
//!
//! SYS_access 系统调用的薄封装。

use core::ffi::{c_char, c_int};
use crate::import::do_syscall;

/// access(filename, amode) — 检查文件的访问权限（使用真实 UID/GID）。
///
/// - amode: F_OK (0) 测试存在性，或 R_OK|W_OK|X_OK 的按位或组合
/// - 允许访问返回 0，拒绝返回 -1 并设置 errno
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn access(filename: *const c_char, amode: c_int) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_access, filename, amode) as c_int }
}
