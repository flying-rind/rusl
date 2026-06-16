//! setuid — 设置调用进程的用户 ID。
//! 对应 musl src/unistd/setuid.c
//!
//! 委托给 __setxid 跨线程同步执行。

use core::ffi::c_int;

/// POSIX `setuid` — 将调用进程的用户 ID 设置为指定值。
/// 内部通过 [`__setxid`] 跨所有线程同步执行 `SYS_setuid` 系统调用。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn setuid(uid: u32) -> c_int {
    super::__setxid(
        crate::syscall::SYS_setuid as c_int,
        uid as c_int,
        -1,
        -1,
    )
}
