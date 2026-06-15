//! getpgid — 获取进程组 ID。
//! 对应 musl src/unistd/getpgid.c
//!
//! SYS_getpgid 系统调用的薄封装，pid 为 0 时查询自身。

use core::ffi::c_int;
use rusl_internal::do_syscall;

/// 获取指定进程 `pid` 的进程组 ID（PGID）。
/// 若 `pid` 为 0，获取调用进程自身的进程组 ID。
/// 成功返回进程组 ID（正整数），失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn getpgid(pid: c_int) -> c_int {
    // 调用 syscall(SYS_getpgid, pid)，通过 syscall_ret 转换返回值
    unsafe { do_syscall!(rusl_internal::syscall::SYS_getpgid, pid) as c_int }
}
