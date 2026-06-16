//! chdir — 将调用进程的当前工作目录改为指定路径。
//! 对应 musl src/unistd/chdir.c
//!
//! SYS_chdir 系统调用的薄封装。

use core::ffi::{c_char, c_int};
use crate::import::do_syscall;

/// chdir(path) — 将当前工作目录变更为 `path` 指定的目录。
///
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn chdir(path: *const c_char) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_chdir, path) as c_int }
}
