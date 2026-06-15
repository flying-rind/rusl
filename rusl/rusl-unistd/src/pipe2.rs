//! pipe2 — 带标志创建管道。
//! 对应 musl src/unistd/pipe2.c
//!
//! SYS_pipe2 系统调用的薄封装。

use core::ffi::c_int;
use rusl_internal::do_syscall;

/// pipe2(fd, flag) — 创建管道并原子性地设置指定标志。
///
/// 支持的 flag：O_CLOEXEC、O_NONBLOCK 的组合。
/// flag=0 时等价于 pipe(fd)。
/// 成功返回 0，出错返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn pipe2(fd: *mut c_int, flag: c_int) -> c_int {
    unsafe { do_syscall!(rusl_internal::syscall::SYS_pipe2, fd, flag) as c_int }
}
