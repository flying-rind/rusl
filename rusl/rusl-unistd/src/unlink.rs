//! unlink — 删除文件名（目录条目）。
//! 对应 musl src/unistd/unlink.c
//!
//! SYS_unlink 系统调用的薄封装。

use core::ffi::{c_char, c_int};
use rusl_internal::do_syscall;

/// unlink(path) — 删除 `path` 指定的文件名。
///
/// 若该文件名是文件的最后一个硬链接且没有进程打开文件，文件数据被删除。
/// 不能用于删除目录（使用 rmdir）。
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn unlink(path: *const c_char) -> c_int {
    unsafe { do_syscall!(rusl_internal::syscall::SYS_unlink, path) as c_int }
}
