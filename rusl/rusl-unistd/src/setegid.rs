//! setegid — 设置调用进程的有效组 ID。
//! 对应 musl src/unistd/setegid.c

use core::ffi::c_int;

/// POSIX `setegid` — 将调用进程的有效组 ID 设置为指定值。
/// 内部通过 [`__setxid`] 调用 `SYS_setresgid(-1, egid, -1)`。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn setegid(egid: u32) -> c_int {
    super::__setxid(
        crate::syscall::SYS_setresgid as c_int,
        -1,
        egid as c_int,
        -1,
    )
}
