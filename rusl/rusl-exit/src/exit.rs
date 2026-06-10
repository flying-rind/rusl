//! `exit` — 标准进程终止函数。
//! 对应 musl `src/exit/exit.c`。
//! 流程：atexit 处理 → fini 析构 → stdio 刷新 → `_Exit(code)`

use core::ffi::c_int;
use core::sync::atomic::{AtomicI32, Ordering};
use rusl_syscall::__syscall1;
use super::sys_consts::SYS_gettid;
use super::_Exit;

// ---------------------------------------------------------------------------
// 弱符号等效 — 默认为空操作, 可由其他模块覆盖
// ---------------------------------------------------------------------------

extern "C" {
    /// 对应 C: `weak_alias(dummy, __stdio_exit)` — stdio 模块提供强定义覆盖。
    fn __stdio_exit();
}

// ---------------------------------------------------------------------------
// .fini_array — ELF 析构函数数组
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "__fini_array_start"]
    static __fini_array_start: unsafe extern "C" fn();

    #[link_name = "__fini_array_end"]
    static __fini_array_end: unsafe extern "C" fn();
}

/// 逆序遍历 .fini_array 调用析构函数。
/// 对应 musl `libc_exit_fini()` (static → weak alias `__libc_exit_fini`)。
/// 不做 `#[no_mangle]` 导出: 动态链接时由 `ldso/dynlink.c` 提供强版本覆盖。
///
/// 与 musl C 版本不同: 不调用 `_fini()` — `.fini_array` 已覆盖所有析构需求，
/// `_fini` 的弱符号语义可用纯 Rust 实现。
unsafe fn libc_exit_fini() {
    let start = &raw const __fini_array_start as *const usize;
    let end = &raw const __fini_array_end as *const usize;
    let mut a = end as usize;
    let step = core::mem::size_of::<unsafe extern "C" fn()>();
    while a > start as usize {
        a -= step;
        let f: unsafe extern "C" fn() = unsafe { core::mem::transmute_copy(&*(a as *const usize)) };
        f();
    }
}

// ---------------------------------------------------------------------------
// exit — 标准进程终止
// ---------------------------------------------------------------------------

/// ISO C `exit` — 以正常状态终止进程。
#[no_mangle]
pub unsafe extern "C" fn exit(code: c_int) -> ! {
    static EXIT_LOCK: AtomicI32 = AtomicI32::new(0);

    let tid = unsafe { __syscall1(SYS_gettid, 0) as c_int };

    let prev = EXIT_LOCK.compare_exchange(0, tid, Ordering::Acquire, Ordering::Relaxed)
        .unwrap_or_else(|v| v);

    if prev == tid {
        // 重入: 同一线程调用了 exit 两次 → crash
        unsafe { core::ptr::null_mut::<u8>().write(0); }
        loop { core::hint::spin_loop(); }
    }
    if prev != 0 {
        // 另一线程正在退出 → 永久阻塞
        loop { core::hint::spin_loop(); }
    }

    super::atexit::__funcs_on_exit();
    unsafe { libc_exit_fini(); }
    unsafe { __stdio_exit(); }
    unsafe { _Exit(code) }
}
