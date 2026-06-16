//! pause — 等待信号。
//! 对应 musl src/unistd/pause.c
//!
//! 调用 SYS_pause 系统调用，阻塞直到收到信号。

use core::ffi::c_int;
use crate::import::do_syscall;

/// 阻塞调用进程，直到收到一个信号并被信号处理器捕获。
/// 返回 -1，errno 设置为 EINTR。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn pause() -> c_int {
    // SAFETY: 系统调用封装，安全性由内核保证。
    unsafe { do_syscall!(crate::syscall::SYS_pause) as c_int }
}
