//! preadv — 从指定偏移分散读取。
//! 对应 musl src/unistd/preadv.c
//!
//! SYS_preadv 系统调用的薄封装。

use core::ffi::c_int;
use crate::import::do_syscall;

/// preadv(fd, iov, count, ofs) — 从文件描述符 `fd` 的偏移 `ofs` 处分批读取。
///
/// 等效于 lseek + readv + 恢复原偏移的原子操作。文件偏移指针不变。
/// 成功返回实际读取字节数，出错返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn preadv(fd: c_int, iov: *const crate::types::iovec, count: c_int, ofs: i64) -> isize {
    unsafe { do_syscall!(crate::syscall::SYS_preadv, fd, iov, count, ofs) as isize }
}
