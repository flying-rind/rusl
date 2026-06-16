//! pwritev — 向指定偏移聚集写入。
//! 对应 musl src/unistd/pwritev.c
//!
//! SYS_pwritev 系统调用的薄封装。

use core::ffi::c_int;
use crate::import::do_syscall;

/// pwritev(fd, iov, count, ofs) — 将多个缓冲区写入文件描述符 `fd` 的偏移 `ofs` 处。
///
/// 文件偏移指针不变。成功返回实际写入字节数，出错返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn pwritev(fd: c_int, iov: *const crate::types::iovec, count: c_int, ofs: i64) -> isize {
    unsafe { do_syscall!(crate::syscall::SYS_pwritev, fd, iov, count, ofs) as isize }
}
