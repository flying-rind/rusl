//! # 线程类型定义
//!
//! musl libc 线程相关类型的 #[repr(C)] 定义。
//! 所有类型的内存布局与 C ABI 完全兼容。

use core::ffi::{c_char, c_int, c_uint, c_ulong, c_void};

// ============================================================================
// 基础类型
// ============================================================================

/// 线程标识符（指向内部 struct pthread 的指针）
pub type pthread_t = *mut c_void;

/// 一次性初始化控制变量
pub type pthread_once_t = c_int;

/// 线程局部存储键
pub type pthread_key_t = c_uint;

/// 时钟标识符
pub type clockid_t = c_int;

// ============================================================================
// 线程属性对象
// ============================================================================

/// 线程属性对象 (musl: union, 56 字节, 对齐至 unsigned long long)
#[repr(C)]
pub struct pthread_attr_t {
    _opaque: [c_ulong; 7],
}

// ============================================================================
// 互斥锁
// ============================================================================

/// 互斥锁 (musl: union, 40 字节)
#[repr(C)]
pub struct pthread_mutex_t {
    _opaque: [c_ulong; 5],
}

/// 互斥锁属性对象 (musl: unsigned int)
#[repr(C)]
pub struct pthread_mutexattr_t {
    pub __attr: c_uint,
}

// ============================================================================
// 读写锁
// ============================================================================

/// 读写锁 (musl: 56 字节)
#[repr(C)]
pub struct pthread_rwlock_t {
    _opaque: [c_ulong; 7],
}

/// 读写锁属性对象 (musl: unsigned __attr[2], 8 字节)
#[repr(C)]
pub struct pthread_rwlockattr_t {
    pub __attr: [c_uint; 2],
}

// ============================================================================
// 条件变量
// ============================================================================

/// 条件变量 (musl: 48 字节)
#[repr(C)]
pub struct pthread_cond_t {
    _opaque: [c_ulong; 6],
}

/// 条件变量属性对象 (musl: unsigned int)
#[repr(C)]
pub struct pthread_condattr_t {
    pub __attr: c_uint,
}

// ============================================================================
// 屏障
// ============================================================================

/// 屏障 (musl: 32 字节)
#[repr(C)]
pub struct pthread_barrier_t {
    _opaque: [c_ulong; 4],
}

/// 屏障属性对象 (musl: unsigned int)
#[repr(C)]
pub struct pthread_barrierattr_t {
    pub __attr: c_uint,
}

// ============================================================================
// 自旋锁
// ============================================================================

/// 自旋锁 (musl: volatile int)
#[repr(C)]
pub struct pthread_spinlock_t {
    pub __lock: c_int,
}

// ============================================================================
// 信号量
// ============================================================================

/// 信号量 (musl: 32 字节)
#[repr(C)]
pub struct sem_t {
    _opaque: [c_ulong; 4],
}

// ============================================================================
// 调度和时间
// ============================================================================

/// 调度参数 (定义于 <sched.h>)
#[repr(C)]
pub struct sched_param {
    pub sched_priority: c_int,
}

/// POSIX 时间结构 (定义于 <time.h>)
#[repr(C)]
pub struct timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

// ============================================================================
// 信号集
// ============================================================================

/// 信号集 (定义于 <signal.h>, 128 字节)
#[repr(C)]
pub struct sigset_t {
    pub __bits: [c_ulong; 16],
}

// ============================================================================
// C11 线程类型
// ============================================================================

/// C11 线程类型 (与 pthread_t 相同)
pub type thrd_t = pthread_t;

/// C11 线程入口函数类型
pub type thrd_start_t = Option<unsafe extern "C" fn(*mut c_void) -> c_int>;

/// C11 线程特定存储键
pub type tss_t = c_uint;

/// C11 TSS 析构函数类型
pub type tss_dtor_t = Option<unsafe extern "C" fn(*mut c_void)>;

/// C11 一次性执行标志
pub type once_flag = c_int;

/// C11 条件变量 (与 pthread_cond_t 相同)
pub type cnd_t = pthread_cond_t;

/// C11 互斥锁 (与 pthread_mutex_t 相同)
pub type mtx_t = pthread_mutex_t;

// ============================================================================
// Sigevent 结构 (用于异步通知)
// ============================================================================

/// sigval 联合体
#[repr(C)]
pub union sigval {
    pub sival_int: c_int,
    pub sival_ptr: *mut c_void,
}

/// sigevent 结构体 (64 字节)
#[repr(C)]
pub struct sigevent {
    pub sigev_value: sigval,
    pub sigev_signo: c_int,
    pub sigev_notify: c_int,
    pub sigev_notify_function: Option<extern "C" fn(sigval)>,
    pub sigev_notify_attributes: *mut pthread_attr_t,
    _pad: [u8; 32],
}

/// siginfo_t 结构体
#[repr(C)]
pub struct siginfo_t {
    pub si_signo: c_int,
    pub si_errno: c_int,
    pub si_code: c_int,
    pub si_pid: c_int,
    pub si_uid: c_uint,
    pub si_value: sigval,
    pub(crate) __pad: [c_char; 88],
}
