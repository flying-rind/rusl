//! setreuid — 原子性地设置调用进程的真实用户 ID 和有效用户 ID。
//! 对应 musl src/unistd/setreuid.c

use core::ffi::c_int;

/// POSIX `setreuid` — 设置真实用户 ID 和有效用户 ID。
/// 参数 `-1`（`(uid_t)-1`）表示不修改对应的 ID。
/// 内部通过 [`__setxid`] 调用 `SYS_setreuid`。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn setreuid(ruid: u32, euid: u32) -> c_int {
    super::__setxid(
        crate::syscall::SYS_setreuid as c_int,
        ruid as c_int,
        euid as c_int,
        -1,
    )
}
