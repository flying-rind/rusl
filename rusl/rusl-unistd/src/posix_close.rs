//! posix_close — POSIX 标准文件描述符关闭函数。
//! 对应 musl src/unistd/posix_close.c
//!
//! 直接委托给 close(fd)，忽略 flags 参数。

use core::ffi::c_int;

/// posix_close(fd, flags) — POSIX 关闭文件描述符。
///
/// 与 close 的区别在于支持 flags 参数。当前实现忽略 flags，
/// 直接委托给 close(fd)。成功返回 0，出错返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn posix_close(fd: c_int, _flags: c_int) -> c_int {
    super::close(fd)
}
