//! write — 向文件描述符写入数据。
//! 对应 musl src/unistd/write.c

use core::ffi::{c_int, c_void};
use rusl_internal::do_syscall;

/// write(fd, buf, count) — 将 `buf` 中最多 `count` 字节写入文件描述符 `fd`。
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn write(
    fd: c_int,
    buf: *const c_void,
    count: usize,
) -> isize {
    // SAFETY: caller guarantees valid fd and buf; the syscall safety is handled by the kernel.
    unsafe { do_syscall!(rusl_internal::syscall::SYS_write, fd, buf, count) as isize }
}

#[cfg(test)]
pub extern "C" fn write(
    _fd: c_int,
    _buf: *const c_void,
    _count: usize,
) -> isize {
    -1
}