// aarch64 (ARM64) Linux syscall ABI
//
// Calling convention:
//   x8  = syscall number
//   x0  = arg1 / return value
//   x1-x5 = arg2-arg6
//   Instruction: svc #0

use core::arch::asm;

macro_rules! syscall {
    ($name:ident, $n:expr, $($arg:ident => $reg:literal),* $(,)?) => {
        #[inline]
        pub unsafe fn $name(n: i64, $($arg: i64),*) -> i64 {
            let ret: i64;
            asm!(
                "svc #0",
                in("x8") n,
                $(in($reg) $arg,)*
                lateout("x0") ret,
                lateout("x1") _,  // all caller-saved may be clobbered
                lateout("x2") _,
                lateout("x3") _,
                lateout("x4") _,
                lateout("x5") _,
                lateout("x6") _,
                lateout("x7") _,
                lateout("x8") _,
                lateout("x9") _,
                lateout("x10") _,
                lateout("x11") _,
                lateout("x12") _,
                lateout("x13") _,
                lateout("x14") _,
                lateout("x15") _,
                lateout("x16") _,
                lateout("x17") _,
                // x18 is reserved
                lateout("x30") _,
                lateout("cc") _,  // condition flags
                options(nostack, preserves_flags),
            );
            ret
        }
    };
}

// Note: aarch64 maps the args to registers sequentially:
// a1=x0(ret), a2=x1, a3=x2, a4=x3, a5=x4, a6=x5
// But x0 overlaps with the return value, so we use lateout for x0.
// We need to move arg1 into x0 before svc.
// For syscall0, x0 just gets the return value.
// For syscall1..6, x0 = a1 is both input and output.

// syscall0: x0 is just output
syscall!(__syscall0, 0,);
// For syscall1..6, x0 is used for arg1
// We need a different approach: use in("x0") for a1
// Actually the problem is that x0 is both input (arg1) and output (ret).
// Let me redefine the macro for aarch64.

// For aarch64, arg1 goes in x0 which is also the return register.
// We need to handle this separately from the generic macro.

#[inline]
pub unsafe fn __syscall1(n: i64, a1: i64) -> i64 {
    let ret: i64;
    asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        lateout("x1") _, lateout("x2") _, lateout("x3") _,
        lateout("x4") _, lateout("x5") _, lateout("x6") _,
        lateout("x7") _, lateout("x8") _, lateout("x9") _,
        lateout("x10") _, lateout("x11") _, lateout("x12") _,
        lateout("x13") _, lateout("x14") _, lateout("x15") _,
        lateout("x16") _, lateout("x17") _, lateout("x30") _,
        lateout("cc") _,
        options(nostack, preserves_flags),
    );
    ret
}

#[inline]
pub unsafe fn __syscall2(n: i64, a1: i64, a2: i64) -> i64 {
    let ret: i64;
    asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        lateout("x2") _, lateout("x3") _, lateout("x4") _,
        lateout("x5") _, lateout("x6") _, lateout("x7") _,
        lateout("x8") _, lateout("x9") _, lateout("x10") _,
        lateout("x11") _, lateout("x12") _, lateout("x13") _,
        lateout("x14") _, lateout("x15") _, lateout("x16") _,
        lateout("x17") _, lateout("x30") _, lateout("cc") _,
        options(nostack, preserves_flags),
    );
    ret
}

#[inline]
pub unsafe fn __syscall3(n: i64, a1: i64, a2: i64, a3: i64) -> i64 {
    let ret: i64;
    asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        lateout("x3") _, lateout("x4") _, lateout("x5") _,
        lateout("x6") _, lateout("x7") _, lateout("x8") _,
        lateout("x9") _, lateout("x10") _, lateout("x11") _,
        lateout("x12") _, lateout("x13") _, lateout("x14") _,
        lateout("x15") _, lateout("x16") _, lateout("x17") _,
        lateout("x30") _, lateout("cc") _,
        options(nostack, preserves_flags),
    );
    ret
}

#[inline]
pub unsafe fn __syscall4(n: i64, a1: i64, a2: i64, a3: i64, a4: i64) -> i64 {
    let ret: i64;
    asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        in("x3") a4,
        lateout("x4") _, lateout("x5") _, lateout("x6") _,
        lateout("x7") _, lateout("x8") _, lateout("x9") _,
        lateout("x10") _, lateout("x11") _, lateout("x12") _,
        lateout("x13") _, lateout("x14") _, lateout("x15") _,
        lateout("x16") _, lateout("x17") _, lateout("x30") _,
        lateout("cc") _,
        options(nostack, preserves_flags),
    );
    ret
}

#[inline]
pub unsafe fn __syscall5(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> i64 {
    let ret: i64;
    asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        in("x3") a4,
        in("x4") a5,
        lateout("x5") _, lateout("x6") _, lateout("x7") _,
        lateout("x8") _, lateout("x9") _, lateout("x10") _,
        lateout("x11") _, lateout("x12") _, lateout("x13") _,
        lateout("x14") _, lateout("x15") _, lateout("x16") _,
        lateout("x17") _, lateout("x30") _, lateout("cc") _,
        options(nostack, preserves_flags),
    );
    ret
}

#[inline]
pub unsafe fn __syscall6(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64, a6: i64) -> i64 {
    let ret: i64;
    asm!(
        "svc #0",
        in("x8") n,
        inlateout("x0") a1 => ret,
        in("x1") a2,
        in("x2") a3,
        in("x3") a4,
        in("x4") a5,
        in("x5") a6,
        lateout("x6") _, lateout("x7") _, lateout("x8") _,
        lateout("x9") _, lateout("x10") _, lateout("x11") _,
        lateout("x12") _, lateout("x13") _, lateout("x14") _,
        lateout("x15") _, lateout("x16") _, lateout("x17") _,
        lateout("x30") _, lateout("cc") _,
        options(nostack, preserves_flags),
    );
    ret
}