//! setresgid — 原子性地设置调用进程的真实、有效和保存 set-group-ID（GNU 扩展）。
//! 对应 musl src/unistd/setresgid.c

use core::ffi::c_int;

/// `setresgid`（GNU 扩展）— 同时设置真实、有效和保存 set-group-ID。
/// 参数 `-1`（`(gid_t)-1`）表示不修改对应的 ID。
/// 内部通过 [`__setxid`] 调用 `SYS_setresgid`。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn setresgid(rgid: u32, egid: u32, sgid: u32) -> c_int {
    super::__setxid(
        crate::syscall::SYS_setresgid as c_int,
        rgid as c_int,
        egid as c_int,
        sgid as c_int,
    )
}
