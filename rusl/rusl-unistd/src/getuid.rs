//! getuid — 获取调用进程的真实用户 ID。
//! 对应 musl src/unistd/getuid.c
//!
//! SYS_getuid 系统调用的薄封装，始终成功。

use core::ffi::c_uint;
use crate::syscall::raw_syscall0;

/// POSIX `getuid` — 返回调用进程的真实用户 ID，总是成功。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn getuid() -> c_uint {
    unsafe { raw_syscall0(crate::syscall::SYS_getuid) as c_uint }
}
