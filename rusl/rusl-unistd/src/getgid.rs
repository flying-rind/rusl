//! getgid — 获取调用进程的真实组 ID。
//! 对应 musl src/unistd/getgid.c
//!
//! SYS_getgid 系统调用的薄封装，始终成功。

use core::ffi::c_uint;
use rusl_internal::syscall::raw_syscall0;

/// POSIX `getgid` — 返回调用进程的真实组 ID，总是成功。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn getgid() -> c_uint {
    unsafe { raw_syscall0(rusl_internal::syscall::SYS_getgid) as c_uint }
}
