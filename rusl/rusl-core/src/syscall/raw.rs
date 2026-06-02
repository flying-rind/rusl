// Syscall dispatch helpers for internal use.
//
// Corresponds to musl's src/internal/syscall.h (defines syscall0..syscall6 macros)

#![allow(dead_code)]  // infrastructure, used by later stages

use crate::arch;

/// Execute a raw syscall and return the kernel's raw result (no errno conversion).

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