//! readv — 分散读取（vectored read）。
//! 对应 musl src/unistd/readv.c
//!
//! SYS_readv 系统调用的薄封装。

use core::ffi::c_int;
use crate::import::do_syscall;

/// readv(fd, iov, count) — 从文件描述符 `fd` 读取数据到多个不连续缓冲区。
///
/// 原子操作，等效于单次 read 到拼接缓冲区但无需数据拷贝。
/// `iov` 指向 count 个 iovec 结构体。成功返回实际读取字节数，
/// 出错返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn readv(fd: c_int, iov: *const crate::types::iovec, count: c_int) -> isize {
    unsafe { do_syscall!(crate::syscall::SYS_readv, fd, iov, count) as isize }
}
