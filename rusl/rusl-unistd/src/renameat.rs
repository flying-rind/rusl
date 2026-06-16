//! renameat — 相对于目录文件描述符原子性地重命名文件/目录。
//! 对应 musl src/unistd/renameat.c
//!
//! SYS_renameat 系统调用的薄封装。

use core::ffi::{c_char, c_int};
use crate::import::do_syscall;

/// renameat(oldfd, old, newfd, new) — 原子性地将 `old` 重命名为 `new`。
///
/// - oldfd: `old` 的基目录 fd 或 AT_FDCWD
/// - newfd: `new` 的基目录 fd 或 AT_FDCWD
///
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn renameat(
    oldfd: c_int,
    old: *const c_char,
    newfd: c_int,
    new: *const c_char,
) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_renameat, oldfd, old, newfd, new) as c_int }
}
