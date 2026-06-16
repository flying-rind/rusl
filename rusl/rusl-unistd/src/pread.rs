//! pread — 从指定偏移读取（原子操作）。
//! 对应 musl src/unistd/pread.c
//!
//! SYS_pread 系统调用的薄封装。

use core::ffi::{c_int, c_void};
use crate::import::do_syscall;

/// pread(fd, buf, size, ofs) — 从文件描述符 `fd` 的偏移 `ofs` 处读取数据。
///
/// 等效于 lseek + read + 恢复原偏移的原子操作，文件偏移指针不变。
/// `ofs` 为 64 位文件偏移（i64）。成功返回实际读取字节数，
/// 出错返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn pread(fd: c_int, buf: *mut c_void, size: usize, ofs: i64) -> isize {
    unsafe { do_syscall!(crate::syscall::SYS_pread, fd, buf, size, ofs) as isize }
}
