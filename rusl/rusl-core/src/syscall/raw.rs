// Syscall dispatch helpers for internal use.
//
// Corresponds to musl's src/internal/syscall.h (defines syscall0..syscall6 macros)

#![allow(dead_code)]  // infrastructure, used by later stages

use crate::arch;

/// Execute a raw syscall and return the kernel's raw result (no errno conversion).
/// Use the `syscall!` macro in syscall_ret.rs for the full conversion.

#[inline]
pub unsafe fn raw_syscall0(nr: i64) -> i64 {
    arch::__syscall0(nr)
}

#[inline]
pub unsafe fn raw_syscall1(nr: i64, a1: i64) -> i64 {
    arch::__syscall1(nr, a1)
}

#[inline]
pub unsafe fn raw_syscall2(nr: i64, a1: i64, a2: i64) -> i64 {
    arch::__syscall2(nr, a1, a2)
}

#[inline]
pub unsafe fn raw_syscall3(nr: i64, a1: i64, a2: i64, a3: i64) -> i64 {
    arch::__syscall3(nr, a1, a2, a3)
}

#[inline]
pub unsafe fn raw_syscall4(nr: i64, a1: i64, a2: i64, a3: i64, a4: i64) -> i64 {
    arch::__syscall4(nr, a1, a2, a3, a4)
}

#[inline]
pub unsafe fn raw_syscall5(nr: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> i64 {
    arch::__syscall5(nr, a1, a2, a3, a4, a5)
}

#[inline]
pub unsafe fn raw_syscall6(nr: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64, a6: i64) -> i64 {
    arch::__syscall6(nr, a1, a2, a3, a4, a5, a6)
}

/// Perform a raw syscall and then apply __syscall_ret conversion.
/// Returns the libc-convention result (sets errno on failure).
#[macro_export]
macro_rules! do_syscall {
    // 0-arg syscall
    ($nr:expr) => {
        $crate::syscall::__syscall_ret(
            $crate::syscall::raw_syscall0($nr) as _
        )
    };
    // 1-arg syscall
    ($nr:expr, $a1:expr) => {
        $crate::syscall::__syscall_ret(
            $crate::syscall::raw_syscall1($nr, $a1 as i64) as _
        )
    };
    // 2-arg syscall
    ($nr:expr, $a1:expr, $a2:expr) => {
        $crate::syscall::__syscall_ret(
            $crate::syscall::raw_syscall2($nr, $a1 as i64, $a2 as i64) as _
        )
    };
    // 3-arg syscall
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr) => {
        $crate::syscall::__syscall_ret(
            $crate::syscall::raw_syscall3($nr, $a1 as i64, $a2 as i64, $a3 as i64) as _
        )
    };
    // 4-arg syscall
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr) => {
        $crate::syscall::__syscall_ret(
            $crate::syscall::raw_syscall4($nr, $a1 as i64, $a2 as i64, $a3 as i64, $a4 as i64) as _
        )
    };
    // 5-arg syscall
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr) => {
        $crate::syscall::__syscall_ret(
            $crate::syscall::raw_syscall5($nr, $a1 as i64, $a2 as i64, $a3 as i64, $a4 as i64, $a5 as i64) as _
        )
    };
    // 6-arg syscall
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr) => {
        $crate::syscall::__syscall_ret(
            $crate::syscall::raw_syscall6($nr, $a1 as i64, $a2 as i64, $a3 as i64, $a4 as i64, $a5 as i64, $a6 as i64) as _
        )
    };
}