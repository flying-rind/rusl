//! `exit` — 标准进程终止函数。
//!
//! 严格对应 musl `src/exit/exit.c`。
//!
//! 流程：atexit 处理 → fini 析构 → stdio 刷新 → `_Exit(code)`

#![allow(non_upper_case_globals)]

use core::ffi::c_int;
use rusl_internal::atomic::{a_cas, a_crash};
use rusl_internal::pthread_impl::__pthread_self;
use crate::_Exit;

// ---------------------------------------------------------------------------
// 弱符号等效：函数指针默认指向空操作，可由其他 crate 覆盖
// 对应 C: weak_alias(dummy, __funcs_on_exit) 等
// ---------------------------------------------------------------------------

extern "C" fn dummy() {}

/// 对应 C: `weak_alias(dummy, __funcs_on_exit)`
pub(crate) static mut __funcs_on_exit: extern "C" fn() = dummy;

/// 对应 C: `weak_alias(dummy, __stdio_exit)`
pub(crate) static mut __stdio_exit: extern "C" fn() = dummy;

// ---------------------------------------------------------------------------
// .fini_array — ELF 析构函数数组
// 对应 C: extern weak hidden void (*const __fini_array_start)(void)
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "__fini_array_start"]
    static __fini_array_start: unsafe extern "C" fn();

    #[link_name = "__fini_array_end"]
    static __fini_array_end: unsafe extern "C" fn();
}

/// 遍历 `.fini_array` 段，按逆序调用所有析构函数。
///
/// 严格对应 musl `exit.c` 中的 `libc_exit_fini()`:
/// ```c
/// static void libc_exit_fini(void)
/// {
///     uintptr_t a = (uintptr_t)&__fini_array_end;
///     for (; a>(uintptr_t)&__fini_array_start; a-=sizeof(void(*)()))
///         (*(void (**)())(a-sizeof(void(*)())))();
///     _fini();
/// }
/// ```
unsafe fn libc_exit_fini() {
    let start = &raw const __fini_array_start as *const usize;
    let end = &raw const __fini_array_end as *const usize;
    // 逆序遍历 fini_array
    let mut a = end as usize;
    let step = core::mem::size_of::<unsafe extern "C" fn()>();
    while a > start as usize {
        a -= step;
        let f: unsafe extern "C" fn() = core::mem::transmute_copy(&*(a as *const usize));
        f();
    }
    // _fini() — 默认指向 dummy，可由链接覆盖
    // 在 Rust 中通过 __fini 函数指针或直接调用链接时覆盖的符号
    extern "C" {
        #[link_name = "_fini"]
        fn _fini();
    }
    unsafe { _fini(); }
}

// ---------------------------------------------------------------------------
// exit — 标准进程终止
// ---------------------------------------------------------------------------

/// ISO C `exit` — 以正常状态终止进程。
///
/// 严格按照 musl 实现：
/// 1. 使用自定义 exit_lock 防止重入和并发退出
/// 2. 调用 `__funcs_on_exit()`（由 atexit 注册的函数）
/// 3. 调用 `__libc_exit_fini()`（遍历 .fini_array 析构函数）
/// 4. 调用 `__stdio_exit()`（刷新 stdio 缓冲区）
/// 5. 调用 `_Exit(code)` 终止进程
#[allow(static_mut_refs)]
#[no_mangle]
pub unsafe extern "C" fn exit(code: c_int) -> ! {
    // 对应 C: static volatile int exit_lock[1];
    static mut EXIT_LOCK: [c_int; 1] = [0];

    // 对应 C: int tid = __pthread_self()->tid;
    let tid = unsafe { (*__pthread_self()).tid };
    // 对应 C: int prev = a_cas(exit_lock, 0, tid);
    let prev = a_cas(EXIT_LOCK.as_mut_ptr(), 0, tid);

    // 对应 C: if (prev == tid) a_crash();
    if prev == tid {
        a_crash();
    }
    // 对应 C: else if (prev) for (;;) __sys_pause();
    if prev != 0 {
        loop {
            // __sys_pause() — 让出 CPU，等待被杀死
            core::hint::spin_loop();
        }
    }

    // 对应 C: __funcs_on_exit();
    unsafe { (__funcs_on_exit)(); }
    // 对应 C: __libc_exit_fini();
    unsafe { libc_exit_fini(); }
    // 对应 C: __stdio_exit();
    unsafe { (__stdio_exit)(); }

    // 对应 C: _Exit(code);
    unsafe { _Exit(code) }
}
