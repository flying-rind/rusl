//! getppid — 获取父进程 ID。
//! 对应 musl src/unistd/getppid.c
//!
//! SYS_getppid 系统调用的薄封装，始终成功。

use core::ffi::c_int;
use rusl_internal::do_syscall;

/// 获取调用进程的父进程 ID（PPID）。
/// 始终成功，返回正整数。父进程退出后该值可能变为 1（init 进程）。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn getppid() -> c_int {
    // 直接返回 syscall(SYS_getppid) 的原始值，因为此调用不会失败
    unsafe { do_syscall!(rusl_internal::syscall::SYS_getppid) as c_int }
}
