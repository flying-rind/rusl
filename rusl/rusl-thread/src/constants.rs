//! # 线程常量定义
//!
//! musl libc 线程相关常量的 Rust 定义。

use core::ffi::c_int;
use core::ffi::c_void;

// ============================================================================
// 线程属性常量
// ============================================================================

pub const PTHREAD_CREATE_JOINABLE: c_int = 0;
pub const PTHREAD_CREATE_DETACHED: c_int = 1;
pub const PTHREAD_INHERIT_SCHED: c_int = 0;
pub const PTHREAD_EXPLICIT_SCHED: c_int = 1;
pub const PTHREAD_SCOPE_SYSTEM: c_int = 0;
pub const PTHREAD_SCOPE_PROCESS: c_int = 1;
pub const PTHREAD_STACK_MIN: usize = 2048;

// ============================================================================
// 互斥锁常量
// ============================================================================

pub const PTHREAD_MUTEX_NORMAL: c_int = 0;
pub const PTHREAD_MUTEX_DEFAULT: c_int = 0;
pub const PTHREAD_MUTEX_RECURSIVE: c_int = 1;
pub const PTHREAD_MUTEX_ERRORCHECK: c_int = 2;
pub const PTHREAD_MUTEX_STALLED: c_int = 0;
pub const PTHREAD_MUTEX_ROBUST: c_int = 1;
pub const PTHREAD_PRIO_NONE: c_int = 0;
pub const PTHREAD_PRIO_INHERIT: c_int = 1;
pub const PTHREAD_PRIO_PROTECT: c_int = 2;

// ============================================================================
// 取消常量
// ============================================================================

pub const PTHREAD_CANCEL_ENABLE: c_int = 0;
pub const PTHREAD_CANCEL_DISABLE: c_int = 1;
pub const PTHREAD_CANCEL_MASKED: c_int = 2;
pub const PTHREAD_CANCEL_DEFERRED: c_int = 0;
pub const PTHREAD_CANCEL_ASYNCHRONOUS: c_int = 1;

/// 线程取消返回值: ((void *)-1)
pub const PTHREAD_CANCELED: *mut c_void = (-1isize) as *mut c_void;

// ============================================================================
// 通用常量
// ============================================================================

pub const PTHREAD_ONCE_INIT: c_int = 0;
pub const PTHREAD_PROCESS_PRIVATE: c_int = 0;
pub const PTHREAD_PROCESS_SHARED: c_int = 1;
pub const PTHREAD_NULL: *mut c_void = core::ptr::null_mut();
pub const PTHREAD_KEYS_MAX: c_int = 128;
pub const PTHREAD_DESTRUCTOR_ITERATIONS: c_int = 4;
pub const SIGCANCEL: c_int = 33;
pub const SIGSYNCCALL: c_int = 34;

// ============================================================================
// 信号量常量
// ============================================================================

pub const SEM_VALUE_MAX: c_int = 0x7FFFFFFF;
pub const SEM_NSEMS_MAX: c_int = 256;
pub const SEM_FAILED: *mut super::types::sem_t = core::ptr::null_mut();

// ============================================================================
// C11 线程常量
// ============================================================================

pub const thrd_success: c_int = 0;
pub const thrd_busy: c_int = 1;
pub const thrd_error: c_int = 2;
pub const thrd_nomem: c_int = 3;
pub const thrd_timedout: c_int = 4;
pub const mtx_plain: c_int = 0;
pub const mtx_recursive: c_int = 1;
pub const mtx_timed: c_int = 2;
pub const ONCE_FLAG_INIT: c_int = 0;
pub const TSS_DTOR_ITERATIONS: c_int = 4;

// ============================================================================
// 信号常量
// ============================================================================

pub const SIG_BLOCK: c_int = 0;
pub const SIG_UNBLOCK: c_int = 1;
pub const SIG_SETMASK: c_int = 2;
pub const SA_SIGINFO: c_int = 4;
pub const SA_RESTART: c_int = 0x10000000;
pub const SA_ONSTACK: c_int = 0x08000000;
pub const _NSIG: c_int = 65;

// ============================================================================
// 时钟常量
// ============================================================================

pub const CLOCK_REALTIME: super::types::clockid_t = 0;

// ============================================================================
// 文件 I/O 常量
// ============================================================================

pub const O_RDONLY: c_int = 0;
pub const O_WRONLY: c_int = 1;
pub const O_RDWR: c_int = 2;
pub const O_CREAT: c_int = 0o100;
pub const O_EXCL: c_int = 0o200;
pub const O_NOFOLLOW: c_int = 0o400000;
pub const O_CLOEXEC: c_int = 0o2000000;
pub const O_NONBLOCK: c_int = 0o4000;

// ============================================================================
// 内存映射常量
// ============================================================================

pub const MAP_SHARED: c_int = 1;
pub const MAP_FAILED: *mut c_void = usize::MAX as *mut c_void;
pub const PROT_READ: c_int = 1;
pub const PROT_WRITE: c_int = 2;

// ============================================================================
// 其他常量
// ============================================================================

pub const F_OK: c_int = 0;
pub const PR_SET_NAME: c_int = 15;
pub const PR_GET_NAME: c_int = 16;

// ============================================================================
// 限制常量
// ============================================================================

pub const SIZE_MAX: usize = usize::MAX;
pub const INT_MAX: c_int = i32::MAX;
pub const NAME_MAX: c_int = 255;

// ============================================================================
// 默认值常量
// ============================================================================

pub const DEFAULT_STACK_SIZE: usize = 131072; // 128KB
pub const DEFAULT_GUARD_SIZE: usize = 8192;   // 8KB
pub const DEFAULT_STACK_MAX: usize = 8 << 20; // 8MB
pub const DEFAULT_GUARD_MAX: usize = 1 << 20; // 1MB
pub const PAGE_SIZE: usize = 4096;

// ============================================================================
// Futex 常量
// ============================================================================

pub const FUTEX_WAIT: c_int = 0;
pub const FUTEX_WAKE: c_int = 1;
pub const FUTEX_LOCK_PI: c_int = 6;
pub const FUTEX_UNLOCK_PI: c_int = 7;
pub const FUTEX_PRIVATE: c_int = 128;
pub const FUTEX_CLOCK_REALTIME: c_int = 256;

// ============================================================================
// 系统调用号 (x86_64)
// ============================================================================

pub const SYS_futex: i64 = 202;
pub const SYS_tkill: i64 = 200;
pub const SYS_close: i64 = 3;
pub const SYS_rt_sigprocmask: i64 = 14;
pub const SYS_gettid: i64 = 186;
pub const SYS_prctl: i64 = 157;

// ============================================================================
// errno 常量
// ============================================================================

pub const EINVAL: c_int = 22;
pub const ENOTSUP: c_int = 95;
pub const ENOMEM: c_int = 12;
pub const EBUSY: c_int = 16;
pub const EAGAIN: c_int = 11;
pub const EPERM: c_int = 1;
pub const EDEADLK: c_int = 35;
pub const EOWNERDEAD: c_int = 130;
pub const ENOTRECOVERABLE: c_int = 131;
pub const ETIMEDOUT: c_int = 110;
pub const EINTR: c_int = 4;
pub const ECANCELED: c_int = 125;
pub const ENOSYS: c_int = 38;
pub const ERANGE: c_int = 34;
pub const EOVERFLOW: c_int = 75;
pub const EMFILE: c_int = 24;
pub const EEXIST: c_int = 17;
pub const ENOENT: c_int = 2;

// ============================================================================
// 互斥锁字段访问索引
// ============================================================================

/// sizeof(size_t) / sizeof(int) 的值 (用于属性字段访问)
pub const __SU: usize = core::mem::size_of::<usize>() / core::mem::size_of::<c_int>();
