//! setregid — 原子性地设置调用进程的真实组 ID 和有效组 ID。
//! 对应 musl src/unistd/setregid.c

use core::ffi::c_int;

/// POSIX `setregid` — 设置真实组 ID 和有效组 ID。
/// 参数 `-1`（`(gid_t)-1`）表示不修改对应的 ID。
/// 内部通过 [`__setxid`] 调用 `SYS_setregid`。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn setregid(rgid: u32, egid: u32) -> c_int {
    super::__setxid(
        rusl_internal::syscall::SYS_setregid as c_int,
        rgid as c_int,
        egid as c_int,
        -1,
    )
}
