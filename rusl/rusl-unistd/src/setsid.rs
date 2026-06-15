//! setsid — 创建新会话。
//! 对应 musl src/unistd/setsid.c
//!
//! SYS_setsid 系统调用的薄封装。
//! 调用进程成为新会话首进程和新进程组首进程，与控制终端断开。

use core::ffi::c_int;
use rusl_internal::do_syscall;

/// 创建新会话，调用进程成为会话首进程和进程组首进程。
/// 成功返回新会话 ID（即当前进程 PID），失败返回 -1 并设置 errno（如 EPERM）。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn setsid() -> c_int {
    // 调用 syscall(SYS_setsid)，通过 syscall_ret 转换返回值
    unsafe { do_syscall!(rusl_internal::syscall::SYS_setsid) as c_int }
}
