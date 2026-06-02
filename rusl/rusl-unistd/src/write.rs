//! write — 向文件描述符写入数据。
//! 对应 musl src/unistd/write.c

use core::ffi::{c_int, c_void};

/// write(fd, buf, count) — 将 `buf` 中最多 `count` 字节写入文件描述符 `fd`。
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn write(
    fd: c_int,
    buf: *const c_void,
    count: usize,
) -> isize {
    use rusl_internal::do_syscall;
    do_syscall!(rusl_core::syscall::SYS_write, fd, buf, count) as isize
}

#[cfg(test)]
pub unsafe extern "C" fn write(
    _fd: c_int,
    _buf: *const c_void,
    _count: usize,
) -> isize {
    -1
}