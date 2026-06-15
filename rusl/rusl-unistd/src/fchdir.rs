//! fchdir — 将调用进程的当前工作目录改为文件描述符所引用的目录。
//! 对应 musl src/unistd/fchdir.c
//!
//! SYS_fchdir 系统调用的薄封装。

use core::ffi::c_int;
use rusl_internal::do_syscall;

/// fchdir(fd) — 将当前工作目录变更为 `fd` 所引用的目录。
///
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn fchdir(fd: c_int) -> c_int {
    unsafe { do_syscall!(rusl_internal::syscall::SYS_fchdir, fd) as c_int }
}
