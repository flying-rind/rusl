//! truncate — 通过路径截断文件。
//! 对应 musl src/unistd/truncate.c
//!
//! SYS_truncate 系统调用的薄封装。

use core::ffi::{c_char, c_int};
use rusl_internal::do_syscall;

/// truncate(path, length) — 将 `path` 指定的普通文件截断为精确 `length` 字节。
///
/// 若文件之前大于 length，超出部分被丢弃；若小于 length，扩展部分填充为零。
/// length 为 64 位文件大小（i64）。成功返回 0，出错返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn truncate(path: *const c_char, length: i64) -> c_int {
    unsafe { do_syscall!(rusl_internal::syscall::SYS_truncate, path, length) as c_int }
}
