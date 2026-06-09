//! `_Exit` — 立即终止进程，不执行任何清理。
//!
//! 严格对应 musl `src/exit/_Exit.c`。
//!
//! ```c
//! _Noreturn void _Exit(int ec)
//! {
//!     __syscall(SYS_exit_group, ec);
//!     for (;;) __syscall(SYS_exit, ec);
//! }
//! ```

use core::ffi::c_int;
use rusl_internal::syscall::{raw_syscall1, SYS_exit, SYS_exit_group};

/// POSIX `_Exit` — 立即终止调用进程。
///
/// 直接通过 `SYS_exit_group` 系统调用终止进程，不调用任何 atexit 或
/// at_quick_exit 注册的函数，不刷新 stdio 缓冲区，不执行任何清理。
///
/// 如果 `SYS_exit_group` 因某些原因返回，则回退到 `SYS_exit`。
#[no_mangle]
pub unsafe extern "C" fn _Exit(ec: c_int) -> ! {
    let c = ec as i64;
    unsafe {
        raw_syscall1(SYS_exit_group, c);
        // 如果 exit_group 返回（极端异常情况），无限循环调用 exit
        loop {
            raw_syscall1(SYS_exit, c);
        }
    }
}
