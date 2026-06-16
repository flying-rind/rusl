//! rmdir — 删除空目录。
//! 对应 musl src/unistd/rmdir.c
//!
//! SYS_rmdir 系统调用的薄封装。

use core::ffi::{c_char, c_int};
use crate::import::do_syscall;

/// rmdir(path) — 删除空目录 `path`。
///
/// 目录必须为空（只能包含 . 和 ..），否则返回 ENOTEMPTY 错误。
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn rmdir(path: *const c_char) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_rmdir, path) as c_int }
}
