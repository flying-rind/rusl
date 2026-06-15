//! setpgrp — 将当前进程设为新进程组首进程。
//! 对应 musl src/unistd/setpgrp.c
//!
//! BSD/XOPEN 历史接口，等价于 setpgid(0, 0)。

use core::ffi::c_int;

/// 将调用进程的进程组 ID 设置为自身 PID，创建以自身为首进程的新进程组。
/// 成功返回新的进程组 ID（即当前进程 PID），失败返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn setpgrp() -> c_int {
    // 委托 setpgid(0, 0)，将当前进程的 PGID 设为其 PID
    super::setpgid(0, 0)
}
