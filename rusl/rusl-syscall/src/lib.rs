//! # rusl-syscall
//!
//! `#![no_std]` 独立的系统调用宏，不依赖任何 rusl 生态 crate。
//!
//! 直接通过内联汇编发起系统调用，并 FFI 调用 musl libc 的 `__syscall_ret`
//! 转换返回值。展开结果与 musl C 源码中的 `syscall()` 宏完全一致。
//!
//! ```c
//! // musl: src/internal/syscall.h
//! #define syscall(...) __syscall_ret(__syscall(__VA_ARGS__))
//! ```
//!
//! ```rust,ignore
//! // rusl-syscall:
//! do_syscall!(SYS_write, fd, buf, count)
//! // 展开为:
//! // __syscall_ret(__syscall3(SYS_write, fd as i64, buf as i64, count as i64) as usize)
//! ```

#![no_std]
#![allow(non_camel_case_types)]

use core::arch::asm;
#[cfg(test)]
use core::panic::PanicInfo;

// ---------------------------------------------------------------------------
// panic handler (仅在单元测试模式下定义,避免与 rusl_core 冲突)
// ---------------------------------------------------------------------------

#[cfg(test)]
#[panic_handler]
fn panic_handler(_info: &PanicInfo) -> ! {
    loop {}
}
// ===========================================================================
// musl __syscall_ret — FFI 链接
//
// 对应 musl src/internal/syscall_ret.c:
//   long __syscall_ret(unsigned long r) {
//       if (r > -4096UL) { errno = -r; return -1; }
//       return r;
//   }
// ===========================================================================

extern "C" {
    /// musl libc 的 `__syscall_ret` — 将内核原始返回值转换为 libc 约定。
    ///
    /// - 若 `r` 在 [-4095, -1] 范围内（即 `r > -4096UL`），
    ///   设置 `errno = -r` 并返回 `-1`。
    /// - 否则原样返回 `r` 的符号扩展值。
    #[link_name = "__syscall_ret"]
    pub fn __syscall_ret(r: usize) -> isize;
}

// ===========================================================================
// 架构级系统调用内联汇编
//
// 对应 musl arch/*/syscall_arch.h 中的 __syscall0..6
// ===========================================================================

// ================================================================
// x86_64 — syscall 指令
// rax = nr (in) / ret (out)
// rdi, rsi, rdx, r10, r8, r9 = arg1..6
// clobber: rcx, r11 (syscall 指令自动覆写)
// ================================================================

#[cfg(target_arch = "x86_64")]
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

#[cfg(target_arch = "x86_64")]
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

#[cfg(target_arch = "x86_64")]
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

#[cfg(target_arch = "x86_64")]
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

#[cfg(target_arch = "x86_64")]
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

#[cfg(target_arch = "x86_64")]
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

#[cfg(target_arch = "x86_64")]
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

// ===========================================================================
// do_syscall! — 展开后与 musl syscall() 宏完全一致
//
// musl C 源码:
//   #define __scc(X) ((long)(X))
//   #define __syscall1(n,a) __syscall1(n,__scc(a))
//   #define syscall(...) __syscall_ret(__syscall(__VA_ARGS__))
//
// 展开示例:
//   syscall(SYS_write, fd, buf, count)
//   → __syscall_ret(__syscall3(SYS_write, (long)fd, (long)buf, (long)count))
//   → __syscall_ret(arch_syscall3(nr, a1, a2, a3))
//
// rusl-syscall:
//   do_syscall!(SYS_write, fd, buf, count)
//   → __syscall_ret(__syscall3(SYS_write, fd as i64, buf as i64, count as i64) as usize)
// ===========================================================================

/// 发起原始系统调用并通过 musl libc 的 `__syscall_ret` 转换返回值。
///
/// 展开后与 musl C 源码中的 `syscall()` 宏产生完全一致的结果：
/// 参数转型 → 架构级 syscall 指令 → `__syscall_ret` 错误码转换。
#[macro_export]
macro_rules! do_syscall {
    // 0-arg
    ($nr:expr) => {{
        let _nr: i64 = $nr;
        let ret = unsafe { $crate::__syscall0(_nr) };
        unsafe { $crate::__syscall_ret(ret as usize) }
    }};
    // 1-arg
    ($nr:expr, $a1:expr) => {{
        let _nr: i64 = $nr;
        let _a1: i64 = $a1 as i64;
        let ret = unsafe { $crate::__syscall1(_nr, _a1) };
        unsafe { $crate::__syscall_ret(ret as usize) }
    }};
    // 2-arg
    ($nr:expr, $a1:expr, $a2:expr) => {{
        let _nr: i64 = $nr;
        let _a1: i64 = $a1 as i64;
        let _a2: i64 = $a2 as i64;
        let ret = unsafe { $crate::__syscall2(_nr, _a1, _a2) };
        unsafe { $crate::__syscall_ret(ret as usize) }
    }};
    // 3-arg
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr) => {{
        let _nr: i64 = $nr;
        let _a1: i64 = $a1 as i64;
        let _a2: i64 = $a2 as i64;
        let _a3: i64 = $a3 as i64;
        let ret = unsafe { $crate::__syscall3(_nr, _a1, _a2, _a3) };
        unsafe { $crate::__syscall_ret(ret as usize) }
    }};
    // 4-arg
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr) => {{
        let _nr: i64 = $nr;
        let _a1: i64 = $a1 as i64;
        let _a2: i64 = $a2 as i64;
        let _a3: i64 = $a3 as i64;
        let _a4: i64 = $a4 as i64;
        let ret = unsafe { $crate::__syscall4(_nr, _a1, _a2, _a3, _a4) };
        unsafe { $crate::__syscall_ret(ret as usize) }
    }};
    // 5-arg
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr) => {{
        let _nr: i64 = $nr;
        let _a1: i64 = $a1 as i64;
        let _a2: i64 = $a2 as i64;
        let _a3: i64 = $a3 as i64;
        let _a4: i64 = $a4 as i64;
        let _a5: i64 = $a5 as i64;
        let ret = unsafe { $crate::__syscall5(_nr, _a1, _a2, _a3, _a4, _a5) };
        unsafe { $crate::__syscall_ret(ret as usize) }
    }};
    // 6-arg
    ($nr:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr) => {{
        let _nr: i64 = $nr;
        let _a1: i64 = $a1 as i64;
        let _a2: i64 = $a2 as i64;
        let _a3: i64 = $a3 as i64;
        let _a4: i64 = $a4 as i64;
        let _a5: i64 = $a5 as i64;
        let _a6: i64 = $a6 as i64;
        let ret = unsafe {
            $crate::__syscall6(_nr, _a1, _a2, _a3, _a4, _a5, _a6)
        };
        unsafe { $crate::__syscall_ret(ret as usize) }
    }};
}