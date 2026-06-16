//! pipe — 创建管道。
//! 对应 musl src/unistd/pipe.c
//!
//! SYS_pipe 系统调用的薄封装。

use core::ffi::c_int;
use crate::import::do_syscall;

/// pipe(fd) — 创建一对单向管道文件描述符。
///
/// `fd` 为指向至少 2 个 c_int 空间的指针：fd[0] 为读端，fd[1] 为写端。
/// 成功返回 0，出错返回 -1 并设置 errno（EMFILE/ENFILE）。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn pipe(fd: *mut c_int) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_pipe, fd) as c_int }
}
