//! thread 模块对外接口 — POSIX 线程 (pthread) 和 C11 线程
//!
//! 当禁用 `rusl` feature 时，通过 extern "C" 声明 musl libc.a 中的符号；
//! 启用 `rusl` feature 时，各符号由 `rusl-thread` crate 提供。

#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_uint, c_ulong, c_void};

// ============================================================================
// 类型定义
// ============================================================================

/// 线程标识符
pub type pthread_t = *mut c_void;

/// 一次性初始化控制变量
pub type pthread_once_t = c_int;

/// 线程局部存储键
pub type pthread_key_t = c_uint;

/// 时钟标识符
pub type clockid_t = c_int;

/// 线程属性对象
#[repr(C)]
pub struct pthread_attr_t { _opaque: [c_ulong; 7] }

/// 互斥锁
#[repr(C)]
pub struct pthread_mutex_t { _opaque: [c_ulong; 5] }

/// 互斥锁属性对象
#[repr(C)]
pub struct pthread_mutexattr_t { pub __attr: c_uint }

/// 读写锁
#[repr(C)]
pub struct pthread_rwlock_t { _opaque: [c_ulong; 7] }

/// 读写锁属性对象
#[repr(C)]
pub struct pthread_rwlockattr_t { pub __attr: [c_uint; 2] }

/// 条件变量
#[repr(C)]
pub struct pthread_cond_t { _opaque: [c_ulong; 6] }

/// 条件变量属性对象
#[repr(C)]
pub struct pthread_condattr_t { pub __attr: c_uint }

/// 屏障
#[repr(C)]
pub struct pthread_barrier_t { _opaque: [c_ulong; 4] }

/// 屏障属性对象
#[repr(C)]
pub struct pthread_barrierattr_t { pub __attr: c_uint }

/// 自旋锁
#[repr(C)]
pub struct pthread_spinlock_t { pub __lock: c_int }

/// 信号量
#[repr(C)]
pub struct sem_t { _opaque: [c_ulong; 4] }

/// 调度参数
#[repr(C)]
pub struct sched_param { pub sched_priority: c_int }

/// 时间结构
#[repr(C)]
pub struct timespec { pub tv_sec: i64, pub tv_nsec: i64 }

/// 信号集
#[repr(C)]
pub struct sigset_t { pub __bits: [c_ulong; 16] }

/// C11 线程类型
pub type thrd_t = pthread_t;
pub type thrd_start_t = Option<unsafe extern "C" fn(*mut c_void) -> c_int>;
pub type tss_t = c_uint;
pub type tss_dtor_t = Option<unsafe extern "C" fn(*mut c_void)>;
pub type once_flag = c_int;
pub type cnd_t = pthread_cond_t;
pub type mtx_t = pthread_mutex_t;

// ============================================================================
// 常量
// ============================================================================

pub const PTHREAD_CREATE_JOINABLE: c_int = 0;
pub const PTHREAD_CREATE_DETACHED: c_int = 1;
pub const PTHREAD_INHERIT_SCHED: c_int = 0;
pub const PTHREAD_EXPLICIT_SCHED: c_int = 1;
pub const PTHREAD_SCOPE_SYSTEM: c_int = 0;
pub const PTHREAD_SCOPE_PROCESS: c_int = 1;
pub const PTHREAD_STACK_MIN: usize = 2048;

pub const PTHREAD_MUTEX_NORMAL: c_int = 0;
pub const PTHREAD_MUTEX_DEFAULT: c_int = 0;
pub const PTHREAD_MUTEX_RECURSIVE: c_int = 1;
pub const PTHREAD_MUTEX_ERRORCHECK: c_int = 2;
pub const PTHREAD_MUTEX_STALLED: c_int = 0;
pub const PTHREAD_MUTEX_ROBUST: c_int = 1;
pub const PTHREAD_PRIO_NONE: c_int = 0;
pub const PTHREAD_PRIO_INHERIT: c_int = 1;
pub const PTHREAD_PRIO_PROTECT: c_int = 2;

pub const PTHREAD_CANCEL_ENABLE: c_int = 0;
pub const PTHREAD_CANCEL_DISABLE: c_int = 1;
pub const PTHREAD_CANCEL_MASKED: c_int = 2;
pub const PTHREAD_CANCEL_DEFERRED: c_int = 0;
pub const PTHREAD_CANCEL_ASYNCHRONOUS: c_int = 1;
pub const PTHREAD_CANCELED: *mut c_void = (-1isize) as *mut c_void;

pub const PTHREAD_ONCE_INIT: c_int = 0;
pub const PTHREAD_PROCESS_PRIVATE: c_int = 0;
pub const PTHREAD_PROCESS_SHARED: c_int = 1;
pub const PTHREAD_NULL: *mut c_void = core::ptr::null_mut();
pub const PTHREAD_KEYS_MAX: c_int = 128;
pub const PTHREAD_DESTRUCTOR_ITERATIONS: c_int = 4;
pub const SIGCANCEL: c_int = 33;
pub const SIGSYNCCALL: c_int = 34;

pub const SEM_VALUE_MAX: c_int = 0x7FFFFFFF;
pub const SEM_NSEMS_MAX: c_int = 256;
pub const SEM_FAILED: *mut sem_t = core::ptr::null_mut();

pub const thrd_success: c_int = 0;
pub const thrd_busy: c_int = 1;
pub const thrd_error: c_int = 2;
pub const thrd_nomem: c_int = 3;
pub const thrd_timedout: c_int = 4;
pub const mtx_plain: c_int = 0;
pub const mtx_recursive: c_int = 1;
pub const mtx_timed: c_int = 2;
pub const ONCE_FLAG_INIT: c_int = 0;

// ============================================================================
// 内部 FFI 声明
// ============================================================================

#[allow(dead_code)]
extern "C" {
    // 线程属性
    #[link_name = "pthread_attr_init"]
    fn musl_pthread_attr_init(a: *mut pthread_attr_t) -> c_int;
    #[link_name = "pthread_attr_destroy"]
    fn musl_pthread_attr_destroy(a: *mut pthread_attr_t) -> c_int;
    #[link_name = "pthread_attr_getdetachstate"]
    fn musl_pthread_attr_getdetachstate(a: *const pthread_attr_t, state: *mut c_int) -> c_int;
    #[link_name = "pthread_attr_getguardsize"]
    fn musl_pthread_attr_getguardsize(a: *const pthread_attr_t, size: *mut usize) -> c_int;
    #[link_name = "pthread_attr_getinheritsched"]
    fn musl_pthread_attr_getinheritsched(a: *const pthread_attr_t, inherit: *mut c_int) -> c_int;
    #[link_name = "pthread_attr_getschedparam"]
    fn musl_pthread_attr_getschedparam(a: *const pthread_attr_t, param: *mut sched_param) -> c_int;
    #[link_name = "pthread_attr_getschedpolicy"]
    fn musl_pthread_attr_getschedpolicy(a: *const pthread_attr_t, policy: *mut c_int) -> c_int;
    #[link_name = "pthread_attr_getscope"]
    fn musl_pthread_attr_getscope(a: *const pthread_attr_t, scope: *mut c_int) -> c_int;
    #[link_name = "pthread_attr_getstack"]
    fn musl_pthread_attr_getstack(a: *const pthread_attr_t, addr: *mut *mut c_void, size: *mut usize) -> c_int;
    #[link_name = "pthread_attr_getstacksize"]
    fn musl_pthread_attr_getstacksize(a: *const pthread_attr_t, size: *mut usize) -> c_int;
    #[link_name = "pthread_attr_setdetachstate"]
    fn musl_pthread_attr_setdetachstate(a: *mut pthread_attr_t, state: c_int) -> c_int;
    #[link_name = "pthread_attr_setguardsize"]
    fn musl_pthread_attr_setguardsize(a: *mut pthread_attr_t, size: usize) -> c_int;
    #[link_name = "pthread_attr_setinheritsched"]
    fn musl_pthread_attr_setinheritsched(a: *mut pthread_attr_t, inherit: c_int) -> c_int;
    #[link_name = "pthread_attr_setschedparam"]
    fn musl_pthread_attr_setschedparam(a: *mut pthread_attr_t, param: *const sched_param) -> c_int;
    #[link_name = "pthread_attr_setschedpolicy"]
    fn musl_pthread_attr_setschedpolicy(a: *mut pthread_attr_t, policy: c_int) -> c_int;
    #[link_name = "pthread_attr_setscope"]
    fn musl_pthread_attr_setscope(a: *mut pthread_attr_t, scope: c_int) -> c_int;
    #[link_name = "pthread_attr_setstack"]
    fn musl_pthread_attr_setstack(a: *mut pthread_attr_t, addr: *mut c_void, size: usize) -> c_int;
    #[link_name = "pthread_attr_setstacksize"]
    fn musl_pthread_attr_setstacksize(a: *mut pthread_attr_t, size: usize) -> c_int;

    // 屏障属性获取
    #[link_name = "pthread_barrierattr_getpshared"]
    fn musl_pthread_barrierattr_getpshared(a: *const pthread_barrierattr_t, pshared: *mut c_int) -> c_int;
    #[link_name = "pthread_condattr_getclock"]
    fn musl_pthread_condattr_getclock(a: *const pthread_condattr_t, clk: *mut clockid_t) -> c_int;
    #[link_name = "pthread_condattr_getpshared"]
    fn musl_pthread_condattr_getpshared(a: *const pthread_condattr_t, pshared: *mut c_int) -> c_int;
    #[link_name = "pthread_mutexattr_getprotocol"]
    fn musl_pthread_mutexattr_getprotocol(a: *const pthread_mutexattr_t, protocol: *mut c_int) -> c_int;
    #[link_name = "pthread_mutexattr_getpshared"]
    fn musl_pthread_mutexattr_getpshared(a: *const pthread_mutexattr_t, pshared: *mut c_int) -> c_int;
    #[link_name = "pthread_mutexattr_getrobust"]
    fn musl_pthread_mutexattr_getrobust(a: *const pthread_mutexattr_t, robust: *mut c_int) -> c_int;
    #[link_name = "pthread_mutexattr_gettype"]
    fn musl_pthread_mutexattr_gettype(a: *const pthread_mutexattr_t, type_: *mut c_int) -> c_int;
    #[link_name = "pthread_rwlockattr_getpshared"]
    fn musl_pthread_rwlockattr_getpshared(a: *const pthread_rwlockattr_t, pshared: *mut c_int) -> c_int;

    // GNU 扩展
    #[link_name = "pthread_getattr_default_np"]
    fn musl_pthread_getattr_default_np(attrp: *mut pthread_attr_t) -> c_int;
    #[link_name = "pthread_setattr_default_np"]
    fn musl_pthread_setattr_default_np(attrp: *const pthread_attr_t) -> c_int;
    #[link_name = "pthread_getattr_np"]
    fn musl_pthread_getattr_np(t: pthread_t, a: *mut pthread_attr_t) -> c_int;

    // 互斥锁
    #[link_name = "pthread_mutexattr_init"]
    fn musl_pthread_mutexattr_init(a: *mut pthread_mutexattr_t) -> c_int;
    #[link_name = "pthread_mutexattr_destroy"]
    fn musl_pthread_mutexattr_destroy(a: *mut pthread_mutexattr_t) -> c_int;
    #[link_name = "pthread_mutexattr_settype"]
    fn musl_pthread_mutexattr_settype(a: *mut pthread_mutexattr_t, type_: c_int) -> c_int;
    #[link_name = "pthread_mutexattr_setpshared"]
    fn musl_pthread_mutexattr_setpshared(a: *mut pthread_mutexattr_t, pshared: c_int) -> c_int;
    #[link_name = "pthread_mutexattr_setrobust"]
    fn musl_pthread_mutexattr_setrobust(a: *mut pthread_mutexattr_t, robust: c_int) -> c_int;
    #[link_name = "pthread_mutexattr_setprotocol"]
    fn musl_pthread_mutexattr_setprotocol(a: *mut pthread_mutexattr_t, protocol: c_int) -> c_int;
    #[link_name = "pthread_mutex_init"]
    fn musl_pthread_mutex_init(m: *mut pthread_mutex_t, a: *const pthread_mutexattr_t) -> c_int;
    #[link_name = "pthread_mutex_destroy"]
    fn musl_pthread_mutex_destroy(mutex: *mut pthread_mutex_t) -> c_int;
    #[link_name = "pthread_mutex_lock"]
    fn musl_pthread_mutex_lock(m: *mut pthread_mutex_t) -> c_int;
    #[link_name = "pthread_mutex_trylock"]
    fn musl_pthread_mutex_trylock(m: *mut pthread_mutex_t) -> c_int;
    #[link_name = "pthread_mutex_timedlock"]
    fn musl_pthread_mutex_timedlock(m: *mut pthread_mutex_t, at: *const timespec) -> c_int;
    #[link_name = "pthread_mutex_unlock"]
    fn musl_pthread_mutex_unlock(m: *mut pthread_mutex_t) -> c_int;
    #[link_name = "pthread_mutex_consistent"]
    fn musl_pthread_mutex_consistent(m: *mut pthread_mutex_t) -> c_int;
    #[link_name = "pthread_mutex_getprioceiling"]
    fn musl_pthread_mutex_getprioceiling(m: *const pthread_mutex_t, ceiling: *mut c_int) -> c_int;
    #[link_name = "pthread_mutex_setprioceiling"]
    fn musl_pthread_mutex_setprioceiling(m: *mut pthread_mutex_t, ceiling: c_int, old: *mut c_int) -> c_int;

    // 读写锁
    #[link_name = "pthread_rwlockattr_init"]
    fn musl_pthread_rwlockattr_init(a: *mut pthread_rwlockattr_t) -> c_int;
    #[link_name = "pthread_rwlockattr_destroy"]
    fn musl_pthread_rwlockattr_destroy(a: *mut pthread_rwlockattr_t) -> c_int;
    #[link_name = "pthread_rwlockattr_setpshared"]
    fn musl_pthread_rwlockattr_setpshared(a: *mut pthread_rwlockattr_t, pshared: c_int) -> c_int;
    #[link_name = "pthread_rwlock_init"]
    fn musl_pthread_rwlock_init(rw: *mut pthread_rwlock_t, a: *const pthread_rwlockattr_t) -> c_int;
    #[link_name = "pthread_rwlock_destroy"]
    fn musl_pthread_rwlock_destroy(rw: *mut pthread_rwlock_t) -> c_int;
    #[link_name = "pthread_rwlock_rdlock"]
    fn musl_pthread_rwlock_rdlock(rw: *mut pthread_rwlock_t) -> c_int;
    #[link_name = "pthread_rwlock_tryrdlock"]
    fn musl_pthread_rwlock_tryrdlock(rw: *mut pthread_rwlock_t) -> c_int;
    #[link_name = "pthread_rwlock_timedrdlock"]
    fn musl_pthread_rwlock_timedrdlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int;
    #[link_name = "pthread_rwlock_wrlock"]
    fn musl_pthread_rwlock_wrlock(rw: *mut pthread_rwlock_t) -> c_int;
    #[link_name = "pthread_rwlock_trywrlock"]
    fn musl_pthread_rwlock_trywrlock(rw: *mut pthread_rwlock_t) -> c_int;
    #[link_name = "pthread_rwlock_timedwrlock"]
    fn musl_pthread_rwlock_timedwrlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int;
    #[link_name = "pthread_rwlock_unlock"]
    fn musl_pthread_rwlock_unlock(rw: *mut pthread_rwlock_t) -> c_int;

    // 条件变量
    #[link_name = "pthread_condattr_init"]
    fn musl_pthread_condattr_init(a: *mut pthread_condattr_t) -> c_int;
    #[link_name = "pthread_condattr_destroy"]
    fn musl_pthread_condattr_destroy(a: *mut pthread_condattr_t) -> c_int;
    #[link_name = "pthread_condattr_setclock"]
    fn musl_pthread_condattr_setclock(a: *mut pthread_condattr_t, clk: clockid_t) -> c_int;
    #[link_name = "pthread_condattr_setpshared"]
    fn musl_pthread_condattr_setpshared(a: *mut pthread_condattr_t, pshared: c_int) -> c_int;
    #[link_name = "pthread_cond_init"]
    fn musl_pthread_cond_init(c: *mut pthread_cond_t, a: *const pthread_condattr_t) -> c_int;
    #[link_name = "pthread_cond_destroy"]
    fn musl_pthread_cond_destroy(c: *mut pthread_cond_t) -> c_int;
    #[link_name = "pthread_cond_wait"]
    fn musl_pthread_cond_wait(c: *mut pthread_cond_t, m: *mut pthread_mutex_t) -> c_int;
    #[link_name = "pthread_cond_timedwait"]
    fn musl_pthread_cond_timedwait(c: *mut pthread_cond_t, m: *mut pthread_mutex_t, ts: *const timespec) -> c_int;
    #[link_name = "pthread_cond_signal"]
    fn musl_pthread_cond_signal(c: *mut pthread_cond_t) -> c_int;
    #[link_name = "pthread_cond_broadcast"]
    fn musl_pthread_cond_broadcast(c: *mut pthread_cond_t) -> c_int;

    // 屏障
    #[link_name = "pthread_barrierattr_init"]
    fn musl_pthread_barrierattr_init(a: *mut pthread_barrierattr_t) -> c_int;
    #[link_name = "pthread_barrierattr_destroy"]
    fn musl_pthread_barrierattr_destroy(a: *mut pthread_barrierattr_t) -> c_int;
    #[link_name = "pthread_barrierattr_setpshared"]
    fn musl_pthread_barrierattr_setpshared(a: *mut pthread_barrierattr_t, pshared: c_int) -> c_int;
    #[link_name = "pthread_barrier_init"]
    fn musl_pthread_barrier_init(b: *mut pthread_barrier_t, a: *const pthread_barrierattr_t, count: c_uint) -> c_int;
    #[link_name = "pthread_barrier_destroy"]
    fn musl_pthread_barrier_destroy(b: *mut pthread_barrier_t) -> c_int;
    #[link_name = "pthread_barrier_wait"]
    fn musl_pthread_barrier_wait(b: *mut pthread_barrier_t) -> c_int;

    // 自旋锁
    #[link_name = "pthread_spin_init"]
    fn musl_pthread_spin_init(s: *mut pthread_spinlock_t, pshared: c_int) -> c_int;
    #[link_name = "pthread_spin_destroy"]
    fn musl_pthread_spin_destroy(s: *mut pthread_spinlock_t) -> c_int;
    #[link_name = "pthread_spin_lock"]
    fn musl_pthread_spin_lock(s: *mut pthread_spinlock_t) -> c_int;
    #[link_name = "pthread_spin_trylock"]
    fn musl_pthread_spin_trylock(s: *mut pthread_spinlock_t) -> c_int;
    #[link_name = "pthread_spin_unlock"]
    fn musl_pthread_spin_unlock(s: *mut pthread_spinlock_t) -> c_int;

    // 一次性初始化
    #[link_name = "pthread_once"]
    fn musl_pthread_once(control: *mut pthread_once_t, init: Option<unsafe extern "C" fn()>) -> c_int;

    // 线程生命周期
    #[link_name = "pthread_create"]
    fn musl_pthread_create(t: *mut pthread_t, a: *const pthread_attr_t, f: Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>, arg: *mut c_void) -> c_int;
    #[link_name = "pthread_exit"]
    fn musl_pthread_exit(retval: *mut c_void);
    #[link_name = "pthread_join"]
    fn musl_pthread_join(t: pthread_t, res: *mut *mut c_void) -> c_int;
    #[link_name = "pthread_detach"]
    fn musl_pthread_detach(t: pthread_t) -> c_int;
    #[link_name = "pthread_self"]
    fn musl_pthread_self() -> pthread_t;
    #[link_name = "pthread_equal"]
    fn musl_pthread_equal(a: pthread_t, b: pthread_t) -> c_int;

    // 线程取消
    #[link_name = "pthread_cancel"]
    fn musl_pthread_cancel(t: pthread_t) -> c_int;
    #[link_name = "pthread_testcancel"]
    fn musl_pthread_testcancel();
    #[link_name = "pthread_setcancelstate"]
    fn musl_pthread_setcancelstate(state: c_int, oldstate: *mut c_int) -> c_int;
    #[link_name = "pthread_setcanceltype"]
    fn musl_pthread_setcanceltype(type_: c_int, oldtype: *mut c_int) -> c_int;

    // 线程调度
    #[link_name = "pthread_getschedparam"]
    fn musl_pthread_getschedparam(t: pthread_t, policy: *mut c_int, param: *mut sched_param) -> c_int;
    #[link_name = "pthread_setschedparam"]
    fn musl_pthread_setschedparam(t: pthread_t, policy: c_int, param: *const sched_param) -> c_int;
    #[link_name = "pthread_setschedprio"]
    fn musl_pthread_setschedprio(t: pthread_t, prio: c_int) -> c_int;
    #[link_name = "pthread_getconcurrency"]
    fn musl_pthread_getconcurrency() -> c_int;
    #[link_name = "pthread_setconcurrency"]
    fn musl_pthread_setconcurrency(val: c_int) -> c_int;
    #[link_name = "pthread_getcpuclockid"]
    fn musl_pthread_getcpuclockid(t: pthread_t, clk: *mut clockid_t) -> c_int;

    // 线程信号
    #[link_name = "pthread_kill"]
    fn musl_pthread_kill(t: pthread_t, sig: c_int) -> c_int;
    #[link_name = "pthread_sigmask"]
    fn musl_pthread_sigmask(how: c_int, set: *const sigset_t, old: *mut sigset_t) -> c_int;

    // 清理处理
    #[link_name = "_pthread_cleanup_push"]
    fn musl_pthread_cleanup_push(cb: *mut c_void, f: Option<unsafe extern "C" fn(*mut c_void)>, x: *mut c_void);
    #[link_name = "_pthread_cleanup_pop"]
    fn musl_pthread_cleanup_pop(cb: *mut c_void, execute: c_int);

    // Fork 处理
    #[link_name = "pthread_atfork"]
    fn musl_pthread_atfork(prepare: Option<unsafe extern "C" fn()>, parent: Option<unsafe extern "C" fn()>, child: Option<unsafe extern "C" fn()>) -> c_int;

    // TSD
    #[link_name = "pthread_key_create"]
    fn musl_pthread_key_create(k: *mut pthread_key_t, dtor: Option<unsafe extern "C" fn(*mut c_void)>) -> c_int;
    #[link_name = "pthread_key_delete"]
    fn musl_pthread_key_delete(k: pthread_key_t) -> c_int;
    #[link_name = "pthread_getspecific"]
    fn musl_pthread_getspecific(k: pthread_key_t) -> *mut c_void;
    #[link_name = "pthread_setspecific"]
    fn musl_pthread_setspecific(k: pthread_key_t, x: *const c_void) -> c_int;

    // 线程命名
    #[link_name = "pthread_setname_np"]
    fn musl_pthread_setname_np(thread: pthread_t, name: *const c_char) -> c_int;
    #[link_name = "pthread_getname_np"]
    fn musl_pthread_getname_np(thread: pthread_t, name: *mut c_char, len: usize) -> c_int;

    // 信号量
    #[link_name = "sem_init"]
    fn musl_sem_init(sem: *mut sem_t, pshared: c_int, value: c_uint) -> c_int;
    #[link_name = "sem_destroy"]
    fn musl_sem_destroy(sem: *mut sem_t) -> c_int;
    #[link_name = "sem_getvalue"]
    fn musl_sem_getvalue(sem: *mut sem_t, valp: *mut c_int) -> c_int;
    #[link_name = "sem_wait"]
    fn musl_sem_wait(sem: *mut sem_t) -> c_int;
    #[link_name = "sem_trywait"]
    fn musl_sem_trywait(sem: *mut sem_t) -> c_int;
    #[link_name = "sem_timedwait"]
    fn musl_sem_timedwait(sem: *mut sem_t, at: *const timespec) -> c_int;
    #[link_name = "sem_post"]
    fn musl_sem_post(sem: *mut sem_t) -> c_int;
    #[link_name = "sem_open"]
    fn musl_sem_open(name: *const c_char, flags: c_int, ...) -> *mut sem_t;
    #[link_name = "sem_close"]
    fn musl_sem_close(sem: *mut sem_t) -> c_int;
    #[link_name = "sem_unlink"]
    fn musl_sem_unlink(name: *const c_char) -> c_int;

    // C11 线程
    #[link_name = "call_once"]
    fn musl_call_once(flag: *mut once_flag, func: Option<unsafe extern "C" fn()>);
    #[link_name = "cnd_init"]
    fn musl_cnd_init(c: *mut cnd_t) -> c_int;
    #[link_name = "cnd_destroy"]
    fn musl_cnd_destroy(c: *mut cnd_t);
    #[link_name = "cnd_wait"]
    fn musl_cnd_wait(c: *mut cnd_t, m: *mut mtx_t) -> c_int;
    #[link_name = "cnd_timedwait"]
    fn musl_cnd_timedwait(c: *mut cnd_t, m: *mut mtx_t, ts: *const timespec) -> c_int;
    #[link_name = "cnd_signal"]
    fn musl_cnd_signal(c: *mut cnd_t) -> c_int;
    #[link_name = "cnd_broadcast"]
    fn musl_cnd_broadcast(c: *mut cnd_t) -> c_int;
    #[link_name = "mtx_init"]
    fn musl_mtx_init(m: *mut mtx_t, type_: c_int) -> c_int;
    #[link_name = "mtx_destroy"]
    fn musl_mtx_destroy(mtx: *mut mtx_t);
    #[link_name = "mtx_lock"]
    fn musl_mtx_lock(m: *mut mtx_t) -> c_int;
    #[link_name = "mtx_timedlock"]
    fn musl_mtx_timedlock(m: *mut mtx_t, ts: *const timespec) -> c_int;
    #[link_name = "mtx_trylock"]
    fn musl_mtx_trylock(m: *mut mtx_t) -> c_int;
    #[link_name = "mtx_unlock"]
    fn musl_mtx_unlock(mtx: *mut mtx_t) -> c_int;
    #[link_name = "thrd_create"]
    fn musl_thrd_create(thr: *mut thrd_t, func: thrd_start_t, arg: *mut c_void) -> c_int;
    #[link_name = "thrd_exit"]
    fn musl_thrd_exit(result: c_int) -> !;
    #[link_name = "thrd_join"]
    fn musl_thrd_join(t: thrd_t, res: *mut c_int) -> c_int;
    #[link_name = "thrd_sleep"]
    fn musl_thrd_sleep(req: *const timespec, rem: *mut timespec) -> c_int;
    #[link_name = "thrd_yield"]
    fn musl_thrd_yield();
    #[link_name = "thrd_current"]
    fn musl_thrd_current() -> pthread_t;
    #[link_name = "thrd_equal"]
    fn musl_thrd_equal(a: pthread_t, b: pthread_t) -> c_int;
    #[link_name = "tss_create"]
    fn musl_tss_create(tss: *mut tss_t, dtor: tss_dtor_t) -> c_int;
    #[link_name = "tss_delete"]
    fn musl_tss_delete(key: tss_t);
    #[link_name = "tss_set"]
    fn musl_tss_set(k: tss_t, x: *mut c_void) -> c_int;
    #[link_name = "tss_get"]
    fn musl_tss_get(k: tss_t) -> *mut c_void;

    // __ 前缀内部符号 (rusl 必须导出)
    #[link_name = "__pthread_rwlock_rdlock"]
    fn musl___pthread_rwlock_rdlock(rw: *mut pthread_rwlock_t) -> c_int;
    #[link_name = "__pthread_rwlock_tryrdlock"]
    fn musl___pthread_rwlock_tryrdlock(rw: *mut pthread_rwlock_t) -> c_int;
    #[link_name = "__pthread_rwlock_timedrdlock"]
    fn musl___pthread_rwlock_timedrdlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int;
    #[link_name = "__pthread_rwlock_wrlock"]
    fn musl___pthread_rwlock_wrlock(rw: *mut pthread_rwlock_t) -> c_int;
    #[link_name = "__pthread_rwlock_trywrlock"]
    fn musl___pthread_rwlock_trywrlock(rw: *mut pthread_rwlock_t) -> c_int;
    #[link_name = "__pthread_rwlock_timedwrlock"]
    fn musl___pthread_rwlock_timedwrlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int;
    #[link_name = "__pthread_rwlock_unlock"]
    fn musl___pthread_rwlock_unlock(rw: *mut pthread_rwlock_t) -> c_int;
    #[link_name = "__pthread_mutex_lock"]
    fn musl___pthread_mutex_lock(m: *mut pthread_mutex_t) -> c_int;
    #[link_name = "__pthread_mutex_trylock"]
    fn musl___pthread_mutex_trylock(m: *mut pthread_mutex_t) -> c_int;
    #[link_name = "__pthread_mutex_timedlock"]
    fn musl___pthread_mutex_timedlock(m: *mut pthread_mutex_t, at: *const timespec) -> c_int;
    #[link_name = "__pthread_mutex_unlock"]
    fn musl___pthread_mutex_unlock(m: *mut pthread_mutex_t) -> c_int;
    #[link_name = "__pthread_once"]
    fn musl___pthread_once(control: *mut pthread_once_t, init: Option<unsafe extern "C" fn()>) -> c_int;
    #[link_name = "__pthread_testcancel"]
    fn musl___pthread_testcancel();
    #[link_name = "__pthread_setcancelstate"]
    fn musl___pthread_setcancelstate(state: c_int, oldstate: *mut c_int) -> c_int;
    #[link_name = "__pthread_self_internal"]
    fn musl___pthread_self_internal() -> pthread_t;
    #[link_name = "__pthread_equal"]
    fn musl___pthread_equal(a: pthread_t, b: pthread_t) -> c_int;
    #[link_name = "__pthread_key_create"]
    fn musl___pthread_key_create(k: *mut pthread_key_t, dtor: Option<unsafe extern "C" fn(*mut c_void)>) -> c_int;
    #[link_name = "__pthread_key_delete"]
    fn musl___pthread_key_delete(k: pthread_key_t) -> c_int;
    #[link_name = "__pthread_getspecific"]
    fn musl___pthread_getspecific(k: pthread_key_t) -> *mut c_void;
    #[link_name = "__pthread_tsd_run_dtors"]
    fn musl___pthread_tsd_run_dtors();
    #[link_name = "__fork_handler"]
    fn musl___fork_handler(who: c_int);
    #[link_name = "__cancel"]
    fn musl___cancel() -> isize;
    #[link_name = "__syscall_cp_c"]
    fn musl___syscall_cp_c(nr: isize, ...) -> isize;
    #[link_name = "__syscall_cp_asm"]
    fn musl___syscall_cp_asm(nr: isize, u: isize, v: isize, w: isize, x: isize, y: isize, z: isize) -> isize;
}

// ============================================================================
// safe 公共封装 — 线程属性
// ============================================================================

pub extern "C" fn pthread_attr_init(a: *mut pthread_attr_t) -> c_int { unsafe { musl_pthread_attr_init(a) } }
pub extern "C" fn pthread_attr_destroy(a: *mut pthread_attr_t) -> c_int { unsafe { musl_pthread_attr_destroy(a) } }
pub extern "C" fn pthread_attr_getdetachstate(a: *const pthread_attr_t, state: *mut c_int) -> c_int { unsafe { musl_pthread_attr_getdetachstate(a, state) } }
pub extern "C" fn pthread_attr_getguardsize(a: *const pthread_attr_t, size: *mut usize) -> c_int { unsafe { musl_pthread_attr_getguardsize(a, size) } }
pub extern "C" fn pthread_attr_getinheritsched(a: *const pthread_attr_t, inherit: *mut c_int) -> c_int { unsafe { musl_pthread_attr_getinheritsched(a, inherit) } }
pub extern "C" fn pthread_attr_getschedparam(a: *const pthread_attr_t, param: *mut sched_param) -> c_int { unsafe { musl_pthread_attr_getschedparam(a, param) } }
pub extern "C" fn pthread_attr_getschedpolicy(a: *const pthread_attr_t, policy: *mut c_int) -> c_int { unsafe { musl_pthread_attr_getschedpolicy(a, policy) } }
pub extern "C" fn pthread_attr_getscope(a: *const pthread_attr_t, scope: *mut c_int) -> c_int { unsafe { musl_pthread_attr_getscope(a, scope) } }
pub extern "C" fn pthread_attr_getstack(a: *const pthread_attr_t, addr: *mut *mut c_void, size: *mut usize) -> c_int { unsafe { musl_pthread_attr_getstack(a, addr, size) } }
pub extern "C" fn pthread_attr_getstacksize(a: *const pthread_attr_t, size: *mut usize) -> c_int { unsafe { musl_pthread_attr_getstacksize(a, size) } }
pub extern "C" fn pthread_attr_setdetachstate(a: *mut pthread_attr_t, state: c_int) -> c_int { unsafe { musl_pthread_attr_setdetachstate(a, state) } }
pub extern "C" fn pthread_attr_setguardsize(a: *mut pthread_attr_t, size: usize) -> c_int { unsafe { musl_pthread_attr_setguardsize(a, size) } }
pub extern "C" fn pthread_attr_setinheritsched(a: *mut pthread_attr_t, inherit: c_int) -> c_int { unsafe { musl_pthread_attr_setinheritsched(a, inherit) } }
pub extern "C" fn pthread_attr_setschedparam(a: *mut pthread_attr_t, param: *const sched_param) -> c_int { unsafe { musl_pthread_attr_setschedparam(a, param) } }
pub extern "C" fn pthread_attr_setschedpolicy(a: *mut pthread_attr_t, policy: c_int) -> c_int { unsafe { musl_pthread_attr_setschedpolicy(a, policy) } }
pub extern "C" fn pthread_attr_setscope(a: *mut pthread_attr_t, scope: c_int) -> c_int { unsafe { musl_pthread_attr_setscope(a, scope) } }
pub extern "C" fn pthread_attr_setstack(a: *mut pthread_attr_t, addr: *mut c_void, size: usize) -> c_int { unsafe { musl_pthread_attr_setstack(a, addr, size) } }
pub extern "C" fn pthread_attr_setstacksize(a: *mut pthread_attr_t, size: usize) -> c_int { unsafe { musl_pthread_attr_setstacksize(a, size) } }

pub extern "C" fn pthread_barrierattr_getpshared(a: *const pthread_barrierattr_t, pshared: *mut c_int) -> c_int { unsafe { musl_pthread_barrierattr_getpshared(a, pshared) } }
pub extern "C" fn pthread_condattr_getclock(a: *const pthread_condattr_t, clk: *mut clockid_t) -> c_int { unsafe { musl_pthread_condattr_getclock(a, clk) } }
pub extern "C" fn pthread_condattr_getpshared(a: *const pthread_condattr_t, pshared: *mut c_int) -> c_int { unsafe { musl_pthread_condattr_getpshared(a, pshared) } }
pub extern "C" fn pthread_mutexattr_getprotocol(a: *const pthread_mutexattr_t, protocol: *mut c_int) -> c_int { unsafe { musl_pthread_mutexattr_getprotocol(a, protocol) } }
pub extern "C" fn pthread_mutexattr_getpshared(a: *const pthread_mutexattr_t, pshared: *mut c_int) -> c_int { unsafe { musl_pthread_mutexattr_getpshared(a, pshared) } }
pub extern "C" fn pthread_mutexattr_getrobust(a: *const pthread_mutexattr_t, robust: *mut c_int) -> c_int { unsafe { musl_pthread_mutexattr_getrobust(a, robust) } }
pub extern "C" fn pthread_mutexattr_gettype(a: *const pthread_mutexattr_t, type_: *mut c_int) -> c_int { unsafe { musl_pthread_mutexattr_gettype(a, type_) } }
pub extern "C" fn pthread_rwlockattr_getpshared(a: *const pthread_rwlockattr_t, pshared: *mut c_int) -> c_int { unsafe { musl_pthread_rwlockattr_getpshared(a, pshared) } }

pub extern "C" fn pthread_getattr_default_np(attrp: *mut pthread_attr_t) -> c_int { unsafe { musl_pthread_getattr_default_np(attrp) } }
pub extern "C" fn pthread_setattr_default_np(attrp: *const pthread_attr_t) -> c_int { unsafe { musl_pthread_setattr_default_np(attrp) } }
pub extern "C" fn pthread_getattr_np(t: pthread_t, a: *mut pthread_attr_t) -> c_int { unsafe { musl_pthread_getattr_np(t, a) } }

// ============================================================================
// safe 公共封装 — 互斥锁
// ============================================================================

pub extern "C" fn pthread_mutexattr_init(a: *mut pthread_mutexattr_t) -> c_int { unsafe { musl_pthread_mutexattr_init(a) } }
pub extern "C" fn pthread_mutexattr_destroy(a: *mut pthread_mutexattr_t) -> c_int { unsafe { musl_pthread_mutexattr_destroy(a) } }
pub extern "C" fn pthread_mutexattr_settype(a: *mut pthread_mutexattr_t, type_: c_int) -> c_int { unsafe { musl_pthread_mutexattr_settype(a, type_) } }
pub extern "C" fn pthread_mutexattr_setpshared(a: *mut pthread_mutexattr_t, pshared: c_int) -> c_int { unsafe { musl_pthread_mutexattr_setpshared(a, pshared) } }
pub extern "C" fn pthread_mutexattr_setrobust(a: *mut pthread_mutexattr_t, robust: c_int) -> c_int { unsafe { musl_pthread_mutexattr_setrobust(a, robust) } }
pub extern "C" fn pthread_mutexattr_setprotocol(a: *mut pthread_mutexattr_t, protocol: c_int) -> c_int { unsafe { musl_pthread_mutexattr_setprotocol(a, protocol) } }
pub extern "C" fn pthread_mutex_init(m: *mut pthread_mutex_t, a: *const pthread_mutexattr_t) -> c_int { unsafe { musl_pthread_mutex_init(m, a) } }
pub extern "C" fn pthread_mutex_destroy(mutex: *mut pthread_mutex_t) -> c_int { unsafe { musl_pthread_mutex_destroy(mutex) } }
pub extern "C" fn pthread_mutex_lock(m: *mut pthread_mutex_t) -> c_int { unsafe { musl_pthread_mutex_lock(m) } }
pub extern "C" fn pthread_mutex_trylock(m: *mut pthread_mutex_t) -> c_int { unsafe { musl_pthread_mutex_trylock(m) } }
pub extern "C" fn pthread_mutex_timedlock(m: *mut pthread_mutex_t, at: *const timespec) -> c_int { unsafe { musl_pthread_mutex_timedlock(m, at) } }
pub extern "C" fn pthread_mutex_unlock(m: *mut pthread_mutex_t) -> c_int { unsafe { musl_pthread_mutex_unlock(m) } }
pub extern "C" fn pthread_mutex_consistent(m: *mut pthread_mutex_t) -> c_int { unsafe { musl_pthread_mutex_consistent(m) } }
pub extern "C" fn pthread_mutex_getprioceiling(m: *const pthread_mutex_t, ceiling: *mut c_int) -> c_int { unsafe { musl_pthread_mutex_getprioceiling(m, ceiling) } }
pub extern "C" fn pthread_mutex_setprioceiling(m: *mut pthread_mutex_t, ceiling: c_int, old: *mut c_int) -> c_int { unsafe { musl_pthread_mutex_setprioceiling(m, ceiling, old) } }

// ============================================================================
// safe 公共封装 — 读写锁
// ============================================================================

pub extern "C" fn pthread_rwlockattr_init(a: *mut pthread_rwlockattr_t) -> c_int { unsafe { musl_pthread_rwlockattr_init(a) } }
pub extern "C" fn pthread_rwlockattr_destroy(a: *mut pthread_rwlockattr_t) -> c_int { unsafe { musl_pthread_rwlockattr_destroy(a) } }
pub extern "C" fn pthread_rwlockattr_setpshared(a: *mut pthread_rwlockattr_t, pshared: c_int) -> c_int { unsafe { musl_pthread_rwlockattr_setpshared(a, pshared) } }
pub extern "C" fn pthread_rwlock_init(rw: *mut pthread_rwlock_t, a: *const pthread_rwlockattr_t) -> c_int { unsafe { musl_pthread_rwlock_init(rw, a) } }
pub extern "C" fn pthread_rwlock_destroy(rw: *mut pthread_rwlock_t) -> c_int { unsafe { musl_pthread_rwlock_destroy(rw) } }
pub extern "C" fn pthread_rwlock_rdlock(rw: *mut pthread_rwlock_t) -> c_int { unsafe { musl_pthread_rwlock_rdlock(rw) } }
pub extern "C" fn pthread_rwlock_tryrdlock(rw: *mut pthread_rwlock_t) -> c_int { unsafe { musl_pthread_rwlock_tryrdlock(rw) } }
pub extern "C" fn pthread_rwlock_timedrdlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int { unsafe { musl_pthread_rwlock_timedrdlock(rw, at) } }
pub extern "C" fn pthread_rwlock_wrlock(rw: *mut pthread_rwlock_t) -> c_int { unsafe { musl_pthread_rwlock_wrlock(rw) } }
pub extern "C" fn pthread_rwlock_trywrlock(rw: *mut pthread_rwlock_t) -> c_int { unsafe { musl_pthread_rwlock_trywrlock(rw) } }
pub extern "C" fn pthread_rwlock_timedwrlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int { unsafe { musl_pthread_rwlock_timedwrlock(rw, at) } }
pub extern "C" fn pthread_rwlock_unlock(rw: *mut pthread_rwlock_t) -> c_int { unsafe { musl_pthread_rwlock_unlock(rw) } }

// ============================================================================
// safe 公共封装 — 条件变量
// ============================================================================

pub extern "C" fn pthread_condattr_init(a: *mut pthread_condattr_t) -> c_int { unsafe { musl_pthread_condattr_init(a) } }
pub extern "C" fn pthread_condattr_destroy(a: *mut pthread_condattr_t) -> c_int { unsafe { musl_pthread_condattr_destroy(a) } }
pub extern "C" fn pthread_condattr_setclock(a: *mut pthread_condattr_t, clk: clockid_t) -> c_int { unsafe { musl_pthread_condattr_setclock(a, clk) } }
pub extern "C" fn pthread_condattr_setpshared(a: *mut pthread_condattr_t, pshared: c_int) -> c_int { unsafe { musl_pthread_condattr_setpshared(a, pshared) } }
pub extern "C" fn pthread_cond_init(c: *mut pthread_cond_t, a: *const pthread_condattr_t) -> c_int { unsafe { musl_pthread_cond_init(c, a) } }
pub extern "C" fn pthread_cond_destroy(c: *mut pthread_cond_t) -> c_int { unsafe { musl_pthread_cond_destroy(c) } }
pub extern "C" fn pthread_cond_wait(c: *mut pthread_cond_t, m: *mut pthread_mutex_t) -> c_int { unsafe { musl_pthread_cond_wait(c, m) } }
pub extern "C" fn pthread_cond_timedwait(c: *mut pthread_cond_t, m: *mut pthread_mutex_t, ts: *const timespec) -> c_int { unsafe { musl_pthread_cond_timedwait(c, m, ts) } }
pub extern "C" fn pthread_cond_signal(c: *mut pthread_cond_t) -> c_int { unsafe { musl_pthread_cond_signal(c) } }
pub extern "C" fn pthread_cond_broadcast(c: *mut pthread_cond_t) -> c_int { unsafe { musl_pthread_cond_broadcast(c) } }

// ============================================================================
// safe 公共封装 — 屏障
// ============================================================================

pub extern "C" fn pthread_barrierattr_init(a: *mut pthread_barrierattr_t) -> c_int { unsafe { musl_pthread_barrierattr_init(a) } }
pub extern "C" fn pthread_barrierattr_destroy(a: *mut pthread_barrierattr_t) -> c_int { unsafe { musl_pthread_barrierattr_destroy(a) } }
pub extern "C" fn pthread_barrierattr_setpshared(a: *mut pthread_barrierattr_t, pshared: c_int) -> c_int { unsafe { musl_pthread_barrierattr_setpshared(a, pshared) } }
pub extern "C" fn pthread_barrier_init(b: *mut pthread_barrier_t, a: *const pthread_barrierattr_t, count: c_uint) -> c_int { unsafe { musl_pthread_barrier_init(b, a, count) } }
pub extern "C" fn pthread_barrier_destroy(b: *mut pthread_barrier_t) -> c_int { unsafe { musl_pthread_barrier_destroy(b) } }
pub extern "C" fn pthread_barrier_wait(b: *mut pthread_barrier_t) -> c_int { unsafe { musl_pthread_barrier_wait(b) } }

// ============================================================================
// safe 公共封装 — 自旋锁
// ============================================================================

pub extern "C" fn pthread_spin_init(s: *mut pthread_spinlock_t, pshared: c_int) -> c_int { unsafe { musl_pthread_spin_init(s, pshared) } }
pub extern "C" fn pthread_spin_destroy(s: *mut pthread_spinlock_t) -> c_int { unsafe { musl_pthread_spin_destroy(s) } }
pub extern "C" fn pthread_spin_lock(s: *mut pthread_spinlock_t) -> c_int { unsafe { musl_pthread_spin_lock(s) } }
pub extern "C" fn pthread_spin_trylock(s: *mut pthread_spinlock_t) -> c_int { unsafe { musl_pthread_spin_trylock(s) } }
pub extern "C" fn pthread_spin_unlock(s: *mut pthread_spinlock_t) -> c_int { unsafe { musl_pthread_spin_unlock(s) } }

// ============================================================================
// safe 公共封装 — 一次性初始化
// ============================================================================

pub extern "C" fn pthread_once(control: *mut pthread_once_t, init: Option<unsafe extern "C" fn()>) -> c_int { unsafe { musl_pthread_once(control, init) } }

// ============================================================================
// safe 公共封装 — 线程生命周期和标识
// ============================================================================

pub extern "C" fn pthread_create(t: *mut pthread_t, a: *const pthread_attr_t, f: Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>, arg: *mut c_void) -> c_int { unsafe { musl_pthread_create(t, a, f, arg) } }
pub extern "C" fn pthread_exit(retval: *mut c_void) { unsafe { musl_pthread_exit(retval) } }
pub extern "C" fn pthread_join(t: pthread_t, res: *mut *mut c_void) -> c_int { unsafe { musl_pthread_join(t, res) } }
pub extern "C" fn pthread_detach(t: pthread_t) -> c_int { unsafe { musl_pthread_detach(t) } }
pub extern "C" fn pthread_self() -> pthread_t { unsafe { musl_pthread_self() } }
pub extern "C" fn pthread_equal(a: pthread_t, b: pthread_t) -> c_int { unsafe { musl_pthread_equal(a, b) } }

// ============================================================================
// safe 公共封装 — 线程取消
// ============================================================================

pub extern "C" fn pthread_cancel(t: pthread_t) -> c_int { unsafe { musl_pthread_cancel(t) } }
pub extern "C" fn pthread_testcancel() { unsafe { musl_pthread_testcancel() } }
pub extern "C" fn pthread_setcancelstate(state: c_int, oldstate: *mut c_int) -> c_int { unsafe { musl_pthread_setcancelstate(state, oldstate) } }
pub extern "C" fn pthread_setcanceltype(type_: c_int, oldtype: *mut c_int) -> c_int { unsafe { musl_pthread_setcanceltype(type_, oldtype) } }

// ============================================================================
// safe 公共封装 — 线程调度
// ============================================================================

pub extern "C" fn pthread_getschedparam(t: pthread_t, policy: *mut c_int, param: *mut sched_param) -> c_int { unsafe { musl_pthread_getschedparam(t, policy, param) } }
pub extern "C" fn pthread_setschedparam(t: pthread_t, policy: c_int, param: *const sched_param) -> c_int { unsafe { musl_pthread_setschedparam(t, policy, param) } }
pub extern "C" fn pthread_setschedprio(t: pthread_t, prio: c_int) -> c_int { unsafe { musl_pthread_setschedprio(t, prio) } }
pub extern "C" fn pthread_getconcurrency() -> c_int { unsafe { musl_pthread_getconcurrency() } }
pub extern "C" fn pthread_setconcurrency(val: c_int) -> c_int { unsafe { musl_pthread_setconcurrency(val) } }
pub extern "C" fn pthread_getcpuclockid(t: pthread_t, clk: *mut clockid_t) -> c_int { unsafe { musl_pthread_getcpuclockid(t, clk) } }

// ============================================================================
// safe 公共封装 — 线程信号
// ============================================================================

pub extern "C" fn pthread_kill(t: pthread_t, sig: c_int) -> c_int { unsafe { musl_pthread_kill(t, sig) } }
pub extern "C" fn pthread_sigmask(how: c_int, set: *const sigset_t, old: *mut sigset_t) -> c_int { unsafe { musl_pthread_sigmask(how, set, old) } }

// ============================================================================
// safe 公共封装 — 清理处理
// ============================================================================

pub extern "C" fn _pthread_cleanup_push(cb: *mut c_void, f: Option<unsafe extern "C" fn(*mut c_void)>, x: *mut c_void) { unsafe { musl_pthread_cleanup_push(cb, f, x) } }
pub extern "C" fn _pthread_cleanup_pop(cb: *mut c_void, execute: c_int) { unsafe { musl_pthread_cleanup_pop(cb, execute) } }

// ============================================================================
// safe 公共封装 — Fork 处理
// ============================================================================

pub extern "C" fn pthread_atfork(prepare: Option<unsafe extern "C" fn()>, parent: Option<unsafe extern "C" fn()>, child: Option<unsafe extern "C" fn()>) -> c_int { unsafe { musl_pthread_atfork(prepare, parent, child) } }

// ============================================================================
// safe 公共封装 — TSD
// ============================================================================

pub extern "C" fn pthread_key_create(k: *mut pthread_key_t, dtor: Option<unsafe extern "C" fn(*mut c_void)>) -> c_int { unsafe { musl_pthread_key_create(k, dtor) } }
pub extern "C" fn pthread_key_delete(k: pthread_key_t) -> c_int { unsafe { musl_pthread_key_delete(k) } }
pub extern "C" fn pthread_getspecific(k: pthread_key_t) -> *mut c_void { unsafe { musl_pthread_getspecific(k) } }
pub extern "C" fn pthread_setspecific(k: pthread_key_t, x: *const c_void) -> c_int { unsafe { musl_pthread_setspecific(k, x) } }

// ============================================================================
// safe 公共封装 — 线程命名
// ============================================================================

pub extern "C" fn pthread_setname_np(thread: pthread_t, name: *const c_char) -> c_int { unsafe { musl_pthread_setname_np(thread, name) } }
pub extern "C" fn pthread_getname_np(thread: pthread_t, name: *mut c_char, len: usize) -> c_int { unsafe { musl_pthread_getname_np(thread, name, len) } }

// ============================================================================
// safe 公共封装 — 信号量
// ============================================================================

pub extern "C" fn sem_init(sem: *mut sem_t, pshared: c_int, value: c_uint) -> c_int { unsafe { musl_sem_init(sem, pshared, value) } }
pub extern "C" fn sem_destroy(sem: *mut sem_t) -> c_int { unsafe { musl_sem_destroy(sem) } }
pub extern "C" fn sem_getvalue(sem: *mut sem_t, valp: *mut c_int) -> c_int { unsafe { musl_sem_getvalue(sem, valp) } }
pub extern "C" fn sem_wait(sem: *mut sem_t) -> c_int { unsafe { musl_sem_wait(sem) } }
pub extern "C" fn sem_trywait(sem: *mut sem_t) -> c_int { unsafe { musl_sem_trywait(sem) } }
pub extern "C" fn sem_timedwait(sem: *mut sem_t, at: *const timespec) -> c_int { unsafe { musl_sem_timedwait(sem, at) } }
pub extern "C" fn sem_post(sem: *mut sem_t) -> c_int { unsafe { musl_sem_post(sem) } }
pub unsafe extern "C" fn sem_open(name: *const c_char, flags: c_int, _: ...) -> *mut sem_t { unimplemented!() }
pub extern "C" fn sem_close(sem: *mut sem_t) -> c_int { unsafe { musl_sem_close(sem) } }
pub extern "C" fn sem_unlink(name: *const c_char) -> c_int { unsafe { musl_sem_unlink(name) } }

// ============================================================================
// safe 公共封装 — C11 线程
// ============================================================================

pub extern "C" fn call_once(flag: *mut once_flag, func: Option<unsafe extern "C" fn()>) { unsafe { musl_call_once(flag, func) } }
pub extern "C" fn cnd_init(c: *mut cnd_t) -> c_int { unsafe { musl_cnd_init(c) } }
pub extern "C" fn cnd_destroy(c: *mut cnd_t) { unsafe { musl_cnd_destroy(c) } }
pub extern "C" fn cnd_wait(c: *mut cnd_t, m: *mut mtx_t) -> c_int { unsafe { musl_cnd_wait(c, m) } }
pub extern "C" fn cnd_timedwait(c: *mut cnd_t, m: *mut mtx_t, ts: *const timespec) -> c_int { unsafe { musl_cnd_timedwait(c, m, ts) } }
pub extern "C" fn cnd_signal(c: *mut cnd_t) -> c_int { unsafe { musl_cnd_signal(c) } }
pub extern "C" fn cnd_broadcast(c: *mut cnd_t) -> c_int { unsafe { musl_cnd_broadcast(c) } }
pub extern "C" fn mtx_init(m: *mut mtx_t, type_: c_int) -> c_int { unsafe { musl_mtx_init(m, type_) } }
pub extern "C" fn mtx_destroy(mtx: *mut mtx_t) { unsafe { musl_mtx_destroy(mtx) } }
pub extern "C" fn mtx_lock(m: *mut mtx_t) -> c_int { unsafe { musl_mtx_lock(m) } }
pub extern "C" fn mtx_timedlock(m: *mut mtx_t, ts: *const timespec) -> c_int { unsafe { musl_mtx_timedlock(m, ts) } }
pub extern "C" fn mtx_trylock(m: *mut mtx_t) -> c_int { unsafe { musl_mtx_trylock(m) } }
pub extern "C" fn mtx_unlock(mtx: *mut mtx_t) -> c_int { unsafe { musl_mtx_unlock(mtx) } }
pub extern "C" fn thrd_create(thr: *mut thrd_t, func: thrd_start_t, arg: *mut c_void) -> c_int { unsafe { musl_thrd_create(thr, func, arg) } }
pub extern "C" fn thrd_exit(result: c_int) -> ! { unsafe { musl_thrd_exit(result) } }
pub extern "C" fn thrd_join(t: thrd_t, res: *mut c_int) -> c_int { unsafe { musl_thrd_join(t, res) } }
pub extern "C" fn thrd_sleep(req: *const timespec, rem: *mut timespec) -> c_int { unsafe { musl_thrd_sleep(req, rem) } }
pub extern "C" fn thrd_yield() { unsafe { musl_thrd_yield() } }
pub extern "C" fn thrd_current() -> pthread_t { unsafe { musl_thrd_current() } }
pub extern "C" fn thrd_equal(a: pthread_t, b: pthread_t) -> c_int { unsafe { musl_thrd_equal(a, b) } }
pub extern "C" fn tss_create(tss: *mut tss_t, dtor: tss_dtor_t) -> c_int { unsafe { musl_tss_create(tss, dtor) } }
pub extern "C" fn tss_delete(key: tss_t) { unsafe { musl_tss_delete(key) } }
pub extern "C" fn tss_set(k: tss_t, x: *mut c_void) -> c_int { unsafe { musl_tss_set(k, x) } }
pub extern "C" fn tss_get(k: tss_t) -> *mut c_void { unsafe { musl_tss_get(k) } }

// ============================================================================
// safe 公共封装 — __ 前缀内部符号
// ============================================================================

pub extern "C" fn __pthread_rwlock_rdlock(rw: *mut pthread_rwlock_t) -> c_int { unsafe { musl___pthread_rwlock_rdlock(rw) } }
pub extern "C" fn __pthread_rwlock_tryrdlock(rw: *mut pthread_rwlock_t) -> c_int { unsafe { musl___pthread_rwlock_tryrdlock(rw) } }
pub extern "C" fn __pthread_rwlock_timedrdlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int { unsafe { musl___pthread_rwlock_timedrdlock(rw, at) } }
pub extern "C" fn __pthread_rwlock_wrlock(rw: *mut pthread_rwlock_t) -> c_int { unsafe { musl___pthread_rwlock_wrlock(rw) } }
pub extern "C" fn __pthread_rwlock_trywrlock(rw: *mut pthread_rwlock_t) -> c_int { unsafe { musl___pthread_rwlock_trywrlock(rw) } }
pub extern "C" fn __pthread_rwlock_timedwrlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int { unsafe { musl___pthread_rwlock_timedwrlock(rw, at) } }
pub extern "C" fn __pthread_rwlock_unlock(rw: *mut pthread_rwlock_t) -> c_int { unsafe { musl___pthread_rwlock_unlock(rw) } }
pub extern "C" fn __pthread_mutex_lock(m: *mut pthread_mutex_t) -> c_int { unsafe { musl___pthread_mutex_lock(m) } }
pub extern "C" fn __pthread_mutex_trylock(m: *mut pthread_mutex_t) -> c_int { unsafe { musl___pthread_mutex_trylock(m) } }
pub extern "C" fn __pthread_mutex_timedlock(m: *mut pthread_mutex_t, at: *const timespec) -> c_int { unsafe { musl___pthread_mutex_timedlock(m, at) } }
pub extern "C" fn __pthread_mutex_unlock(m: *mut pthread_mutex_t) -> c_int { unsafe { musl___pthread_mutex_unlock(m) } }
pub extern "C" fn __pthread_once(control: *mut pthread_once_t, init: Option<unsafe extern "C" fn()>) -> c_int { unsafe { musl___pthread_once(control, init) } }
pub extern "C" fn __pthread_testcancel() { unsafe { musl___pthread_testcancel() } }
pub extern "C" fn __pthread_setcancelstate(state: c_int, oldstate: *mut c_int) -> c_int { unsafe { musl___pthread_setcancelstate(state, oldstate) } }
pub extern "C" fn __pthread_self_internal() -> pthread_t { unsafe { musl___pthread_self_internal() } }
pub extern "C" fn __pthread_equal(a: pthread_t, b: pthread_t) -> c_int { unsafe { musl___pthread_equal(a, b) } }
pub extern "C" fn __pthread_key_create(k: *mut pthread_key_t, dtor: Option<unsafe extern "C" fn(*mut c_void)>) -> c_int { unsafe { musl___pthread_key_create(k, dtor) } }
pub extern "C" fn __pthread_key_delete(k: pthread_key_t) -> c_int { unsafe { musl___pthread_key_delete(k) } }
pub extern "C" fn __pthread_getspecific(k: pthread_key_t) -> *mut c_void { unsafe { musl___pthread_getspecific(k) } }
pub extern "C" fn __pthread_tsd_run_dtors() { unsafe { musl___pthread_tsd_run_dtors() } }
pub extern "C" fn __fork_handler(who: c_int) { unsafe { musl___fork_handler(who) } }
pub extern "C" fn __cancel() -> isize { unsafe { musl___cancel() } }
pub unsafe extern "C" fn __syscall_cp_c(_nr: isize, _: ...) -> isize { unimplemented!() }
pub extern "C" fn __syscall_cp_asm(nr: isize, u: isize, v: isize, w: isize, x: isize, y: isize, z: isize) -> isize { unsafe { musl___syscall_cp_asm(nr, u, v, w, x, y, z) } }
