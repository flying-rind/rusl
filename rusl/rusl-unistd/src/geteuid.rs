//! geteuid — 获取调用进程的有效用户 ID。
//! 对应 musl src/unistd/geteuid.c
//!
//! SYS_geteuid 系统调用的薄封装，始终成功。

use core::ffi::c_uint;
use crate::syscall::raw_syscall0;

/// POSIX `geteuid` — 返回调用进程的有效用户 ID，总是成功。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn geteuid() -> c_uint {
    unsafe { raw_syscall0(crate::syscall::SYS_geteuid) as c_uint }
}
