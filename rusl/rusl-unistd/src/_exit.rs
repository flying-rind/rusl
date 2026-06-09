//! `_exit` — POSIX 进程终止（不执行清理）。
//!
//! 严格对应 musl `src/unistd/_exit.c`。
//!
//! ```c
//! _Noreturn void _exit(int status)
//! {
//!     _Exit(status);
//! }
//! ```

use core::ffi::c_int;

extern "C" {
    fn _Exit(code: c_int) -> !;
}

/// POSIX `_exit` — 立即终止调用进程，不执行清理。
///
/// 委托给 [`_Exit`]，与 musl 实现完全一致。
#[no_mangle]
pub unsafe extern "C" fn _exit(status: c_int) -> ! {
    unsafe { _Exit(status) }
}
