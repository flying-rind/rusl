//! setgid — 设置调用进程的组 ID。
//! 对应 musl src/unistd/setgid.c

use core::ffi::c_int;

/// POSIX `setgid` — 将调用进程的组 ID 设置为指定值。
/// 内部通过 [`__setxid`] 跨所有线程同步执行 `SYS_setgid` 系统调用。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn setgid(gid: u32) -> c_int {
    super::__setxid(
        crate::syscall::SYS_setgid as c_int,
        gid as c_int,
        -1,
        -1,
    )
}
