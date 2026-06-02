// Syscall dispatch macro — moved from rusl-core.
//
// Corresponds to musl's src/internal/syscall.h (defines syscall0..syscall6 macros)

/// Perform a raw syscall and then apply __syscall_ret conversion.
/// Returns the libc-convention result (sets errno on failure).
#[macro_export]
macro_rules! do_syscall {
    // 0-arg syscall
    ($nr:expr) => {
        $crate::syscall_ret::__syscall_ret(
            ::rusl_core::syscall::raw_syscall0($nr) as _
        )
    };
    // 1-arg syscall
    ($nr:expr, $a1:expr) => {
        $crate::syscall_ret::__syscall_ret(
            ::rusl_core::syscall::raw_syscall1($nr, $a1 as i64) as _
        )
    };
    // 2-arg syscall
    ($nr:expr, $a1:expr, $a2:expr) => {
        $crate::syscall_ret::__syscall_ret(
            ::rusl_core::syscall::raw_syscall2($nr, $a1 as i64, $a2 as i64) as _
        )
    };
    // 3-arg syscall
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr) => {
        $crate::syscall_ret::__syscall_ret(
            ::rusl_core::syscall::raw_syscall3($nr, $a1 as i64, $a2 as i64, $a3 as i64) as _
        )
    };
    // 4-arg syscall
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr) => {
        $crate::syscall_ret::__syscall_ret(
            ::rusl_core::syscall::raw_syscall4($nr, $a1 as i64, $a2 as i64, $a3 as i64, $a4 as i64) as _
        )
    };
    // 5-arg syscall
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr) => {
        $crate::syscall_ret::__syscall_ret(
            ::rusl_core::syscall::raw_syscall5($nr, $a1 as i64, $a2 as i64, $a3 as i64, $a4 as i64, $a5 as i64) as _
        )
    };
    // 6-arg syscall
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr) => {
        $crate::syscall_ret::__syscall_ret(
            ::rusl_core::syscall::raw_syscall6($nr, $a1 as i64, $a2 as i64, $a3 as i64, $a4 as i64, $a5 as i64, $a6 as i64) as _
        )
    };
}