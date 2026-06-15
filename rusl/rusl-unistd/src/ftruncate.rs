//! ftruncate — 通过文件描述符截断文件。
//! 对应 musl src/unistd/ftruncate.c
//!
//! SYS_ftruncate 系统调用的薄封装。

use core::ffi::c_int;
use rusl_internal::do_syscall;

/// ftruncate(fd, length) — 将文件描述符 `fd` 引用的文件截断为精确 `length` 字节。
///
/// 与 truncate 类似，但通过 fd 操作。需要 fd 以写模式打开。
/// length 为 64 位文件大小（i64）。成功返回 0，出错返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn ftruncate(fd: c_int, length: i64) -> c_int {
    unsafe { do_syscall!(rusl_internal::syscall::SYS_ftruncate, fd, length) as c_int }
}
