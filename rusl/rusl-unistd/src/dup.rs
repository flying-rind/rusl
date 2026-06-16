//! dup — 复制文件描述符。
//! 对应 musl src/unistd/dup.c
//!
//! SYS_dup 系统调用的薄封装。

use core::ffi::c_int;
use crate::import::do_syscall;

/// dup(fd) — 复制文件描述符 `fd`，返回最低可用编号的新文件描述符。
///
/// 新旧描述符共享同一内核文件描述（文件偏移、状态标志），
/// 但不共享 close-on-exec 标志（新 fd 的 FD_CLOEXEC 被清除）。
/// 成功返回新 fd（非负），出错返回 -1 并设置 errno（EBADF/EMFILE）。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn dup(fd: c_int) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_dup, fd) as c_int }
}
