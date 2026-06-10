//! `_Exit` — 立即终止进程，不执行任何清理。
//! 对应 musl `src/exit/_Exit.c`。

use core::ffi::c_int;
use rusl_syscall::__syscall1;
use super::sys_consts::{SYS_exit, SYS_exit_group};

#[no_mangle]
pub unsafe extern "C" fn _Exit(ec: c_int) -> ! {
    let c = ec as i64;
    unsafe {
        __syscall1(SYS_exit_group, c);
        loop {
            __syscall1(SYS_exit, c);
        }
    }
}
