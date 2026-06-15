//! fchown — 通过文件描述符改变文件的所有者和/或所属组。
//! 对应 musl src/unistd/fchown.c
//!
//! SYS_fchown 系统调用的薄封装。

use core::ffi::c_int;
use rusl_internal::do_syscall;

/// fchown(fd, uid, gid) — 通过文件描述符 `fd` 改变文件所有者/组。
///
/// uid 或 gid 为 -1 时，对应属性保持不变。
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn fchown(fd: c_int, uid: u32, gid: u32) -> c_int {
    unsafe { do_syscall!(rusl_internal::syscall::SYS_fchown, fd, uid, gid) as c_int }
}
