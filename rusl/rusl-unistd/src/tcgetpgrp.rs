//! tcgetpgrp — 获取终端的前台进程组 ID。
//! 对应 musl src/unistd/tcgetpgrp.c
//!
//! 通过 TIOCGPGRP ioctl 控制命令实现。

use core::ffi::c_int;
use crate::syscall::raw_syscall3;

/// TIOCGPGRP — 获取前台进程组的 ioctl 命令
const TIOCGPGRP: i64 = 0x540F;

/// POSIX `tcgetpgrp` — 返回与 `fd` 关联的终端的前台进程组 ID。
///
/// 通过 `TIOCGPGRP` ioctl 控制命令实现。
/// 成功返回前台进程组 ID，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn tcgetpgrp(fd: c_int) -> c_int {
    let mut pgrp: c_int = 0;
    let r = unsafe {
        raw_syscall3(
            crate::syscall::SYS_ioctl,
            fd as i64,
            TIOCGPGRP,
            &mut pgrp as *mut c_int as i64,
        )
    };
    if r < 0 {
        // 设置 errno 并返回 -1
        let _ = crate::syscall::__syscall_ret(r as u64);
        -1
    } else {
        pgrp
    }
}
