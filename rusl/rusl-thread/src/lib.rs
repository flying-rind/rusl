//! # rusl-thread
//!
//! `#![no_std]` Rust 实现的 musl libc 线程 (pthread) 模块。
//!
//! 提供 POSIX 线程接口 (pthread_*) 和 C11 线程接口 (thrd_*, mtx_*, cnd_*, tss_*, call_once)
//! 以及 POSIX 信号量接口 (sem_*)。

#![no_std]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]
#![feature(custom_test_frameworks)]
#![feature(c_variadic)]
#![test_runner(rusl_core::runner)]
#![reexport_test_harness_main = "test_main"]
#![no_main]

extern crate rusl_core;
extern crate rusl_syscall;

// ============================================================================
// 内部模块
// ============================================================================

pub(crate) mod import;

// do_syscall 从 import 模块导出
pub use crate::import::do_syscall;

// ============================================================================
// 公共模块
// ============================================================================

pub mod types;
pub mod constants;

mod attr;
mod mutex;
mod rwlock;
mod cond;
mod barrier;
mod spin;
mod once;
mod thread;
mod cancel;
mod sched;
mod signal;
mod cleanup;
mod atfork;
mod tsd;
mod name;
mod semaphore;
mod c11;
mod internals;

// ============================================================================
// 类型重导出
// ============================================================================

pub use types::*;

// ============================================================================
// 常量重导出
// ============================================================================

pub use constants::*;

// ============================================================================
// 函数重导出 — 线程属性
// ============================================================================

pub use attr::{
    pthread_attr_init, pthread_attr_destroy,
    pthread_attr_getdetachstate, pthread_attr_getguardsize,
    pthread_attr_getinheritsched, pthread_attr_getschedparam,
    pthread_attr_getschedpolicy, pthread_attr_getscope,
    pthread_attr_getstack, pthread_attr_getstacksize,
    pthread_attr_setdetachstate, pthread_attr_setguardsize,
    pthread_attr_setinheritsched, pthread_attr_setschedparam,
    pthread_attr_setschedpolicy, pthread_attr_setscope,
    pthread_attr_setstack, pthread_attr_setstacksize,
    pthread_barrierattr_getpshared,
    pthread_condattr_getclock, pthread_condattr_getpshared,
    pthread_mutexattr_getprotocol, pthread_mutexattr_getpshared,
    pthread_mutexattr_getrobust, pthread_mutexattr_gettype,
    pthread_rwlockattr_getpshared,
    pthread_getattr_default_np, pthread_setattr_default_np,
    pthread_getattr_np,
};

// ============================================================================
// 函数重导出 — 互斥锁
// ============================================================================

pub use mutex::{
    pthread_mutexattr_init, pthread_mutexattr_destroy,
    pthread_mutexattr_settype, pthread_mutexattr_setpshared,
    pthread_mutexattr_setrobust, pthread_mutexattr_setprotocol,
    pthread_mutex_init, pthread_mutex_destroy,
    pthread_mutex_lock, pthread_mutex_trylock,
    pthread_mutex_timedlock, pthread_mutex_unlock,
    pthread_mutex_consistent,
    pthread_mutex_getprioceiling, pthread_mutex_setprioceiling,
};

// ============================================================================
// 函数重导出 — 读写锁
// ============================================================================

pub use rwlock::{
    pthread_rwlockattr_init, pthread_rwlockattr_destroy,
    pthread_rwlockattr_setpshared,
    pthread_rwlock_init, pthread_rwlock_destroy,
    pthread_rwlock_rdlock, pthread_rwlock_tryrdlock,
    pthread_rwlock_timedrdlock,
    pthread_rwlock_wrlock, pthread_rwlock_trywrlock,
    pthread_rwlock_timedwrlock, pthread_rwlock_unlock,
};

// ============================================================================
// 函数重导出 — 条件变量
// ============================================================================

pub use cond::{
    pthread_condattr_init, pthread_condattr_destroy,
    pthread_condattr_setclock, pthread_condattr_setpshared,
    pthread_cond_init, pthread_cond_destroy,
    pthread_cond_wait, pthread_cond_timedwait,
    pthread_cond_signal, pthread_cond_broadcast,
};

// ============================================================================
// 函数重导出 — 屏障
// ============================================================================

pub use barrier::{
    pthread_barrierattr_init, pthread_barrierattr_destroy,
    pthread_barrierattr_setpshared,
    pthread_barrier_init, pthread_barrier_destroy,
    pthread_barrier_wait,
};

// ============================================================================
// 函数重导出 — 自旋锁
// ============================================================================

pub use spin::{
    pthread_spin_init, pthread_spin_destroy,
    pthread_spin_lock, pthread_spin_trylock,
    pthread_spin_unlock,
};

// ============================================================================
// 函数重导出 — 一次性初始化
// ============================================================================

pub use once::pthread_once;

// ============================================================================
// 函数重导出 — 线程生命周期和标识
// ============================================================================

pub use thread::{
    pthread_create, pthread_exit, pthread_join, pthread_detach,
    pthread_self, pthread_equal,
};

// ============================================================================
// 函数重导出 — 线程取消
// ============================================================================

pub use cancel::{
    pthread_cancel, pthread_testcancel,
    pthread_setcancelstate, pthread_setcanceltype,
};

// ============================================================================
// 函数重导出 — 线程调度
// ============================================================================

pub use sched::{
    pthread_getschedparam, pthread_setschedparam,
    pthread_setschedprio,
    pthread_getconcurrency, pthread_setconcurrency,
    pthread_getcpuclockid,
};

// ============================================================================
// 函数重导出 — 线程信号
// ============================================================================

pub use signal::{
    pthread_kill, pthread_sigmask,
};

// ============================================================================
// 函数重导出 — 清理处理
// ============================================================================

pub use cleanup::{
    _pthread_cleanup_push, _pthread_cleanup_pop,
};

// ============================================================================
// 函数重导出 — Fork 处理
// ============================================================================

pub use atfork::pthread_atfork;

// ============================================================================
// 函数重导出 — TSD
// ============================================================================

pub use tsd::{
    pthread_key_create, pthread_key_delete,
    pthread_getspecific, pthread_setspecific,
};

// ============================================================================
// 函数重导出 — 线程命名
// ============================================================================

pub use name::{
    pthread_setname_np, pthread_getname_np,
};

// ============================================================================
// 函数重导出 — 信号量
// ============================================================================

pub use semaphore::{
    sem_init, sem_destroy, sem_getvalue,
    sem_wait, sem_trywait, sem_timedwait, sem_post,
    sem_open, sem_close, sem_unlink,
};

// ============================================================================
// 函数重导出 — C11 线程
// ============================================================================

pub use c11::{
    call_once,
    cnd_init, cnd_destroy, cnd_wait, cnd_timedwait,
    cnd_signal, cnd_broadcast,
    mtx_init, mtx_destroy, mtx_lock, mtx_timedlock,
    mtx_trylock, mtx_unlock,
    thrd_create, thrd_exit, thrd_join, thrd_sleep,
    thrd_yield, thrd_current, thrd_equal,
    tss_create, tss_delete, tss_set, tss_get,
};

// ============================================================================
// 函数重导出 — musl __ 内部符号
// ============================================================================

pub use internals::{
    __pthread_rwlock_rdlock, __pthread_rwlock_tryrdlock,
    __pthread_rwlock_timedrdlock,
    __pthread_rwlock_wrlock, __pthread_rwlock_trywrlock,
    __pthread_rwlock_timedwrlock, __pthread_rwlock_unlock,
    __pthread_mutex_lock, __pthread_mutex_trylock,
    __pthread_mutex_timedlock, __pthread_mutex_unlock,
    __pthread_once, __pthread_testcancel,
    __pthread_setcancelstate,
    __pthread_self_internal, __pthread_equal,
    __pthread_key_create, __pthread_key_delete,
    __pthread_getspecific, __pthread_tsd_run_dtors,
    __pthread_tsd_size, __pthread_tsd_main,
    __fork_handler, __cancel,
    __syscall_cp_c, __syscall_cp_asm,
    __sem_open_lockptr,
};

// ============================================================================
// 测试入口
// ============================================================================

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start(_argc: i32, _argv: *const *const u8) -> i32 {
    test_main();
    0
}
