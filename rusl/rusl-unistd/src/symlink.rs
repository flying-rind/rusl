//! symlink — 创建符号链接。
//! 对应 musl src/unistd/symlink.c
//!
//! SYS_symlink 系统调用的薄封装。

use core::ffi::{c_char, c_int};
use crate::import::do_syscall;

/// symlink(existing, new) — 创建符号链接 `new` 指向目标 `existing`。
///
/// 目标不需要存在。符号链接是包含目标路径字符串的特殊文件类型。
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn symlink(existing: *const c_char, new: *const c_char) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_symlink, existing, new) as c_int }
}
