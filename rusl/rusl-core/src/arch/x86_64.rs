// x86_64 Linux syscall ABI
//
// Calling convention:
//   rax = syscall number (input) / return value (output)
//   rdi = arg1, rsi = arg2, rdx = arg3
//   r10 = arg4, r8  = arg5, r9  = arg6
//   Clobbered: rcx, r11 (the syscall instruction itself clobbers these)
//
// Each function returns the raw kernel return value.
// Negative values in [-4095, -1] represent errno; __syscall_ret handles the conversion.

use core::arch::asm;

#[inline]
pub unsafe fn __syscall0(n: i64) -> i64 {
    let ret: i64;
    asm!(
        "syscall",
        inlateout("rax") n => ret,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack, preserves_flags),
    );
    ret
}

#[inline]
pub unsafe fn __syscall1(n: i64, a1: i64) -> i64 {
    let ret: i64;
    asm!(
        "syscall",
        inlateout("rax") n => ret,
        in("rdi") a1,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack, preserves_flags),
    );
    ret
}

#[inline]
pub unsafe fn __syscall2(n: i64, a1: i64, a2: i64) -> i64 {
    let ret: i64;
    asm!(
        "syscall",
        inlateout("rax") n => ret,
        in("rdi") a1,
        in("rsi") a2,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack, preserves_flags),
    );
    ret
}

#[inline]
pub unsafe fn __syscall3(n: i64, a1: i64, a2: i64, a3: i64) -> i64 {
    let ret: i64;
    asm!(
        "syscall",
        inlateout("rax") n => ret,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack, preserves_flags),
    );
    ret
}

#[inline]
pub unsafe fn __syscall4(n: i64, a1: i64, a2: i64, a3: i64, a4: i64) -> i64 {
    let ret: i64;
    asm!(
        "syscall",
        inlateout("rax") n => ret,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        in("r10") a4,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack, preserves_flags),
    );
    ret
}

#[inline]
pub unsafe fn __syscall5(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> i64 {
    let ret: i64;
    asm!(
        "syscall",
        inlateout("rax") n => ret,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        in("r10") a4,
        in("r8") a5,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack, preserves_flags),
    );
    ret
}

#[inline]
pub unsafe fn __syscall6(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64, a6: i64) -> i64 {
    let ret: i64;
    asm!(
        "syscall",
        inlateout("rax") n => ret,
        in("rdi") a1,
        in("rsi") a2,
        in("rdx") a3,
        in("r10") a4,
        in("r8") a5,
        in("r9") a6,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack, preserves_flags),
    );
    ret
}