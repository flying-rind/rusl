//! pwrite — 向指定偏移写入（原子操作）。
//! 对应 musl src/unistd/pwrite.c
//!
//! SYS_pwrite 系统调用的薄封装。

use core::ffi::{c_int, c_void};
use crate::import::do_syscall;

/// pwrite(fd, buf, size, ofs) — 向文件描述符 `fd` 的偏移 `ofs` 处写入数据。
///
/// 文件偏移指针不变。成功返回实际写入字节数，
/// 出错返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn pwrite(fd: c_int, buf: *const c_void, size: usize, ofs: i64) -> isize {
    unsafe { do_syscall!(crate::syscall::SYS_pwrite, fd, buf, size, ofs) as isize }
}
