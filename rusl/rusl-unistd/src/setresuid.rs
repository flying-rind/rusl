//! setresuid — 原子性地设置调用进程的真实、有效和保存 set-user-ID（GNU 扩展）。
//! 对应 musl src/unistd/setresuid.c

use core::ffi::c_int;

/// `setresuid`（GNU 扩展）— 同时设置真实、有效和保存 set-user-ID。
/// 参数 `-1`（`(uid_t)-1`）表示不修改对应的 ID。
/// 内部通过 [`__setxid`] 调用 `SYS_setresuid`。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn setresuid(ruid: u32, euid: u32, suid: u32) -> c_int {
    super::__setxid(
        rusl_internal::syscall::SYS_setresuid as c_int,
        ruid as c_int,
        euid as c_int,
        suid as c_int,
    )
}
