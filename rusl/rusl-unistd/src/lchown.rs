//! lchown — 改变符号链接自身（而非其目标）的所有者和/或所属组。
//! 对应 musl src/unistd/lchown.c
//!
//! SYS_lchown 系统调用的薄封装。

use core::ffi::{c_char, c_int};
use rusl_internal::do_syscall;

/// lchown(path, uid, gid) — 改变符号链接 `path` 自身（而非目标）的所有者/组。
///
/// uid 或 gid 为 -1 时，对应属性保持不变。
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn lchown(path: *const c_char, uid: u32, gid: u32) -> c_int {
    unsafe { do_syscall!(rusl_internal::syscall::SYS_lchown, path, uid, gid) as c_int }
}
