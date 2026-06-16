//! symlinkat — 相对于目录文件描述符创建符号链接。
//! 对应 musl src/unistd/symlinkat.c
//!
//! SYS_symlinkat 系统调用的薄封装。

use core::ffi::{c_char, c_int};
use crate::import::do_syscall;

/// symlinkat(existing, fd, new) — 相对于目录 `fd` 创建符号链接 `new` 指向 `existing`。
///
/// - fd: 新链接的基目录 fd 或 AT_FDCWD
///
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn symlinkat(existing: *const c_char, fd: c_int, new: *const c_char) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_symlinkat, existing, fd, new) as c_int }
}
