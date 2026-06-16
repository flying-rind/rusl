//! read — 从文件描述符读取数据。
//! 对应 musl src/unistd/read.c

use core::ffi::{c_int, c_void};
use crate::import::do_syscall;

/// read(fd, buf, count) — 将文件描述符 `fd` 中至多 `count` 字节读入缓冲区 `buf`。
///
/// 该调用是 `SYS_read` 系统调用的薄封装。返回实际读取的字节数，0 表示 EOF，
/// 出错时返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize {
    unsafe { do_syscall!(crate::syscall::SYS_read, fd, buf, count) as isize }
}
