//! link — 创建文件的硬链接。
//! 对应 musl src/unistd/link.c
//!
//! SYS_link 系统调用的薄封装。

use core::ffi::{c_char, c_int};
use crate::import::do_syscall;

/// link(existing, new) — 为 `existing` 创建硬链接 `new`。
///
/// 两个路径名指向同一 inode，共享所有数据和元数据。
/// `new` 必须不存在，且两者不能在不同文件系统上。
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn link(existing: *const c_char, new: *const c_char) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_link, existing, new) as c_int }
}
