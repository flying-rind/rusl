//! unlinkat — 相对于目录文件描述符删除文件名。
//! 对应 musl src/unistd/unlinkat.c
//!
//! SYS_unlinkat 系统调用的薄封装。

use core::ffi::{c_char, c_int};
use crate::import::do_syscall;

/// unlinkat(fd, path, flag) — 相对于目录 `fd` 删除 `path`。
///
/// - fd: 目录文件描述符或 AT_FDCWD
/// - flag: 0 或 AT_REMOVEDIR
///
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn unlinkat(fd: c_int, path: *const c_char, flag: c_int) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_unlinkat, fd, path, flag) as c_int }
}
