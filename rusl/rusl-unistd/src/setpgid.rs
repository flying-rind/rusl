//! setpgid — 设置进程组 ID。
//! 对应 musl src/unistd/setpgid.c
//!
//! SYS_setpgid 系统调用的薄封装。pid=0 表示当前进程，pgid=0 表示使用 pid 自身值。

use core::ffi::c_int;
use rusl_internal::do_syscall;

/// 将进程 `pid` 的进程组 ID 设置为 `pgid`。
/// 若 `pid` 为 0，操作调用进程自身；若 `pgid` 为 0，使用 `pid` 的值。
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn setpgid(pid: c_int, pgid: c_int) -> c_int {
    // 调用 syscall(SYS_setpgid, pid, pgid)，通过 syscall_ret 转换返回值
    unsafe { do_syscall!(rusl_internal::syscall::SYS_setpgid, pid, pgid) as c_int }
}
