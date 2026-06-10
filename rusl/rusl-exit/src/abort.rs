//! abort — 异常终止当前进程。
//! 对应 musl src/exit/abort.c

use rusl_syscall::{__syscall0, __syscall1, __syscall2};
use super::sys_consts::{SYS_exit_group, SYS_getpid, SYS_kill, SIGABRT};

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn abort() -> ! {
    unsafe {
        let pid = __syscall0(SYS_getpid);
        __syscall2(SYS_kill, pid, SIGABRT);
        __syscall1(SYS_exit_group, 127);
    }
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
pub extern "C" fn abort() -> ! {
    panic!("abort() called in test mode");
}
