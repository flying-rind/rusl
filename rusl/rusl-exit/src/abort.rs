//! abort — 异常终止当前进程。
//! 对应 musl src/exit/abort.c
//!
//! 【极简版本】仅做 SIGABRT + exit_group 兜底。
//! 未实现: 信号屏蔽/锁/sigaction 重置/tkill 等 musl 完整逻辑。
//! 待信号/线程基础设施就绪后替换为完整版本。

use core::ffi::c_int;

/// 发送 SIGABRT 信号给当前进程，若信号被忽略或处理器返回，
/// 则通过 `SYS_exit_group(127)` 强制退出。从不返回。
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn abort() -> ! {
    let pid = rusl_core::syscall::raw_syscall0(rusl_core::syscall::SYS_getpid);
    // SYS_kill(pid, SIGABRT=6): 向当前进程发送 SIGABRT
    rusl_core::syscall::raw_syscall2(rusl_core::syscall::SYS_kill, pid, 6);

    // 若 SIGABRT 被用户处理器捕获且返回，或被忽略，强制终止
    rusl_core::syscall::raw_syscall1(rusl_core::syscall::SYS_exit_group, 127);

    // 保险: 死循环 + 段错误确保绝不返回
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
pub unsafe extern "C" fn abort() -> ! {
    panic!("abort() called in test mode");
}