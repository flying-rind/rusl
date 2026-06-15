//! writev — 聚集写入（vectored write）。
//! 对应 musl src/unistd/writev.c
//!
//! SYS_writev 系统调用的薄封装。

use core::ffi::c_int;
use rusl_internal::do_syscall;

/// writev(fd, iov, count) — 将多个不连续缓冲区的数据原子写入文件描述符 `fd`。
///
/// 等效于单次 write 但无需数据拼接。成功返回实际写入字节数，
/// 出错返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn writev(fd: c_int, iov: *const crate::types::iovec, count: c_int) -> isize {
    unsafe { do_syscall!(rusl_internal::syscall::SYS_writev, fd, iov, count) as isize }
}
