//! getpgrp — 获取调用进程的进程组 ID。
//! 对应 musl src/unistd/getpgrp.c
//!
//! 等价于 getpgid(0)，以 pid=0 调用 SYS_getpgid，始终成功。

use core::ffi::c_int;
use crate::import::do_syscall;

/// 获取调用进程的进程组 ID（PGID）。
/// 等价于 `getpgid(0)`，始终成功，返回正整数。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn getpgrp() -> c_int {
    // 以参数 pid=0 调用 SYS_getpgid，直接返回原始值（始终成功）
    unsafe { do_syscall!(crate::syscall::SYS_getpgid, 0) as c_int }
}
