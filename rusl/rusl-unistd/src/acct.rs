//! acct — 启用或禁用进程记账。
//! 对应 musl src/unistd/acct.c
//!
//! SYS_acct 系统调用的薄封装（GNU 扩展，非 POSIX）。
//! 需要 CAP_SYS_PACCT 权限。

use core::ffi::{c_char, c_int};
use crate::import::do_syscall;

/// 启用或禁用进程记账。
/// `filename` 非 NULL 时启用记账并将信息写入指定文件；
/// `filename` 为 NULL 时禁用记账。
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn acct(filename: *const c_char) -> c_int {
    // 调用 syscall(SYS_acct, filename)，通过 syscall_ret 转换返回值
    unsafe { do_syscall!(crate::syscall::SYS_acct, filename) as c_int }
}
