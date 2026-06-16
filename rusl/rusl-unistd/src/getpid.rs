//! getpid — 获取当前进程 ID。
//! 对应 musl src/unistd/getpid.c
//!
//! SYS_getpid 系统调用的薄封装，始终成功。

use core::ffi::c_int;
use crate::import::do_syscall;

/// 获取调用进程的进程 ID（PID）。
/// 始终成功，返回正整数，不设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn getpid() -> c_int {
    // 直接返回 syscall(SYS_getpid) 的原始值，因为此调用不会失败
    unsafe { do_syscall!(crate::syscall::SYS_getpid) as c_int }
}
