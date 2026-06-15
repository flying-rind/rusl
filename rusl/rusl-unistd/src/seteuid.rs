//! seteuid — 设置调用进程的有效用户 ID。
//! 对应 musl src/unistd/seteuid.c

use core::ffi::c_int;

/// POSIX `seteuid` — 将调用进程的有效用户 ID 设置为指定值。
/// 内部通过 [`__setxid`] 调用 `SYS_setresuid(-1, euid, -1)`。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn seteuid(euid: u32) -> c_int {
    super::__setxid(
        rusl_internal::syscall::SYS_setresuid as c_int,
        -1,
        euid as c_int,
        -1,
    )
}
