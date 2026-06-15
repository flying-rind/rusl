//! tcsetpgrp — 设置终端的前台进程组。
//! 对应 musl src/unistd/tcsetpgrp.c
//!
//! 通过 TIOCSPGRP ioctl 控制命令实现。

use core::ffi::c_int;
use rusl_internal::do_syscall;

/// TIOCSPGRP — 设置前台进程组的 ioctl 命令
const TIOCSPGRP: i64 = 0x5410;

/// POSIX `tcsetpgrp` — 将 `fd` 关联的终端的前台进程组设置为 `pgrp`。
///
/// 通过 `TIOCSPGRP` ioctl 控制命令实现。
/// 调用进程必须与终端属于同一会话且持有终端的控制权。
/// 成功返回 0，失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn tcsetpgrp(fd: c_int, pgrp: c_int) -> c_int {
    unsafe {
        do_syscall!(
            rusl_internal::syscall::SYS_ioctl,
            fd,
            TIOCSPGRP,
            &pgrp as *const c_int
        ) as c_int
    }
}
