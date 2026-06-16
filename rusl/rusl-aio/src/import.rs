//! 声明所有依赖其他模块的接口
//!
//! 当开启rusl feature时，依赖其他rusl-xxx crate；
//! 否则使用extern "C"链接musl libc的C实现。

#![allow(dead_code, unused_imports, unused_variables)]

use core::ffi::{c_int, c_uint, c_void, c_char, c_long, c_ulong};

// ============================================================================
// 内存分配接口 (通过 C ABI 链接 — 与 musl libc 和 rusl_malloc 均兼容)
// ============================================================================

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn calloc(n: usize, size: usize) -> *mut c_void;
    pub fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
}

// ============================================================================
// 文件 I/O 接口 (来自 rusl-unistd 或 musl libc)
// ============================================================================

#[cfg(feature = "rusl")]
pub use rusl_unistd::{
    read, write, pread, pwrite,
    fsync, fdatasync, lseek, close,
    getpid, getuid,
};

#[cfg(not(feature = "rusl"))]
pub use crate::import::unistd::{
    read, write, pread, pwrite,
    fsync, fdatasync, lseek, close,
    getpid, getuid,
};

#[cfg(not(feature = "rusl"))]
mod unistd {
    use core::ffi::{c_int, c_void, c_uint};
    extern "C" {
        pub fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
        pub fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
        pub fn pread(fd: c_int, buf: *mut c_void, size: usize, ofs: i64) -> isize;
        pub fn pwrite(fd: c_int, buf: *const c_void, size: usize, ofs: i64) -> isize;
        pub fn fsync(fd: c_int) -> c_int;
        pub fn fdatasync(fd: c_int) -> c_int;
        pub fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
        pub fn close(fd: c_int) -> c_int;
        pub fn getpid() -> c_int;
        pub fn getuid() -> c_uint;
    }
}

// ============================================================================
// 内存操作接口 (来自 rusl-string 或 musl libc)
// ============================================================================

#[cfg(feature = "rusl")]
pub use rusl_string::memcpy;

#[cfg(not(feature = "rusl"))]
pub use crate::import::string::memcpy;

#[cfg(not(feature = "rusl"))]
mod string {
    use core::ffi::c_void;
    extern "C" {
        pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    }
}

// ============================================================================
// 系统调用 / 内部辅助 (来自 rusl-internal 或 rusl-syscall)
// ============================================================================

#[cfg(feature = "rusl")]
pub use rusl_internal::do_syscall;

#[cfg(not(feature = "rusl"))]
pub use rusl_syscall::do_syscall;

// ============================================================================
// errno 接口 (来自 rusl-errno 或 musl libc)
// ============================================================================

#[cfg(feature = "rusl")]
pub use rusl_errno::__errno_location;

#[cfg(not(feature = "rusl"))]
pub use crate::import::errno::__errno_location;

#[cfg(not(feature = "rusl"))]
mod errno {
    use core::ffi::c_int;
    extern "C" {
        #[link_name = "__errno_location"]
        fn musl_errno_location() -> *mut c_int;
    }
    pub extern "C" fn __errno_location() -> *mut c_int { unsafe { musl_errno_location() } }
}

// ============================================================================
// 线程管理接口 (pthread) — 尚未有 rusl-pthread crate, 使用 extern "C"
// ============================================================================

pub mod pthread {
    use core::ffi::{c_int, c_uint, c_void};

    // pthread 类型占位 (repr(C) 不透明)
    #[repr(C)]
    pub struct pthread_t { pub(crate) _opaque: [u8; 8] }

    #[repr(C)]
    pub struct pthread_attr_t { pub(crate) _opaque: [u8; 64] }

    #[repr(C)]
    pub struct pthread_mutex_t { pub(crate) _opaque: [u8; 40] }

    #[repr(C)]
    pub struct pthread_mutexattr_t { pub(crate) _opaque: [u8; 8] }

    #[repr(C)]
    pub struct pthread_cond_t { pub(crate) _opaque: [u8; 48] }

    #[repr(C)]
    pub struct pthread_condattr_t { pub(crate) _opaque: [u8; 8] }

    #[repr(C)]
    pub struct pthread_rwlock_t { pub(crate) _opaque: [u8; 56] }

    #[repr(C)]
    pub struct pthread_rwlockattr_t { pub(crate) _opaque: [u8; 16] }

    #[repr(C)]
    pub struct sem_t { pub(crate) _opaque: [u8; 32] }

    // 常量
    pub const PTHREAD_CREATE_DETACHED: c_int = 1;

    /// musl 内部清理处理器链表节点
    #[repr(C)]
    pub struct __ptcb {
        pub __next: *mut __ptcb,
        pub __f: Option<extern "C" fn(*mut c_void)>,
        pub __x: *mut c_void,
    }

    pub const PTHREAD_CANCEL_DISABLE: c_int = 0;
    pub const PTHREAD_CANCEL_ENABLE: c_int = 1;

    extern "C" {
        pub fn pthread_create(
            thr: *mut pthread_t,
            attr: *const pthread_attr_t,
            start: extern "C" fn(*mut c_void) -> *mut c_void,
            arg: *mut c_void,
        ) -> c_int;
        pub fn pthread_cancel(thr: pthread_t) -> c_int;
        pub fn pthread_attr_init(attr: *mut pthread_attr_t) -> c_int;
        pub fn pthread_attr_setstacksize(attr: *mut pthread_attr_t, size: usize) -> c_int;
        pub fn pthread_attr_setguardsize(attr: *mut pthread_attr_t, size: usize) -> c_int;
        pub fn pthread_attr_setdetachstate(attr: *mut pthread_attr_t, state: c_int) -> c_int;
        pub fn pthread_mutex_lock(m: *mut pthread_mutex_t) -> c_int;
        pub fn pthread_mutex_unlock(m: *mut pthread_mutex_t) -> c_int;
        pub fn pthread_mutex_init(m: *mut pthread_mutex_t, attr: *const pthread_mutexattr_t) -> c_int;
        pub fn pthread_cond_wait(cond: *mut pthread_cond_t, m: *mut pthread_mutex_t) -> c_int;
        pub fn pthread_cond_broadcast(cond: *mut pthread_cond_t) -> c_int;
        pub fn pthread_cond_init(cond: *mut pthread_cond_t, attr: *const pthread_condattr_t) -> c_int;
        pub fn pthread_rwlock_rdlock(rw: *mut pthread_rwlock_t) -> c_int;
        pub fn pthread_rwlock_wrlock(rw: *mut pthread_rwlock_t) -> c_int;
        pub fn pthread_rwlock_tryrdlock(rw: *mut pthread_rwlock_t) -> c_int;
        pub fn pthread_rwlock_unlock(rw: *mut pthread_rwlock_t) -> c_int;
        pub fn pthread_rwlock_init(
            rw: *mut pthread_rwlock_t,
            attr: *const pthread_rwlockattr_t,
        ) -> c_int;
        pub fn pthread_sigmask(
            how: c_int,
            set: *const super::sigset_t,
            old: *mut super::sigset_t,
        ) -> c_int;
        pub fn pthread_testcancel();
        /// POSIX 标准: 获取当前线程的 pthread_t
        pub fn pthread_self() -> *mut c_void;
        pub fn sem_init(sem: *mut sem_t, pshared: c_int, value: c_uint) -> c_int;
        pub fn sem_post(sem: *mut sem_t) -> c_int;
        pub fn sem_wait(sem: *mut sem_t) -> c_int;
        /// pthread 取消状态管理
        pub fn pthread_setcancelstate(state: c_int, oldstate: *mut c_int) -> c_int;
        /// musl 内部: 清理处理器压栈/出栈
        pub fn _pthread_cleanup_push(cb: *mut __ptcb, f: extern "C" fn(*mut c_void), x: *mut c_void);
        pub fn _pthread_cleanup_pop(cb: *mut __ptcb, execute: c_int);
    }
}

// ============================================================================
// 信号处理接口 (signal) — 尚未有 rusl-signal crate, 使用 extern "C"
// ============================================================================

/// 信号集类型 (repr(C) 不透明占位)
#[repr(C)]
pub struct sigset_t {
    pub(crate) _opaque: [u8; 128],
}

/// sigevent 结构体 (POSIX 异步 I/O 通知)
///
/// 布局与 musl 1.2.6 `<signal.h>` 的 `struct sigevent` 完全一致 (64 字节).
#[repr(C)]
pub struct sigevent {
    pub sigev_value: sigval,
    pub sigev_signo: c_int,
    pub sigev_notify: c_int,
    pub sigev_notify_function: Option<extern "C" fn(sigval)>,
    pub sigev_notify_attributes: *mut pthread::pthread_attr_t,
    _pad: [u8; 32], // musl: __sev_fields union padding → total = 64
}

/// sigval 联合体
#[repr(C)]
pub union sigval {
    pub sival_int: c_int,
    pub sival_ptr: *mut c_void,
}

/// siginfo_t 结构体 (简化占位)
#[repr(C)]
pub struct siginfo_t {
    pub si_signo: c_int,
    pub si_errno: c_int,
    pub si_code: c_int,
    pub si_pid: c_int,
    pub si_uid: c_uint,
    pub si_value: sigval,
    pub(crate) __pad: [c_char; 88], // 填充，与 musl ABI 对齐
}

// 信号常量
pub const SIG_BLOCK: c_int = 0;
pub const SIG_SETMASK: c_int = 2;
pub const SI_ASYNCIO: c_int = -4;
pub const SIGEV_NONE: c_int = 1;
pub const SIGEV_SIGNAL: c_int = 2;
pub const SIGEV_THREAD: c_int = 3;

extern "C" {
    pub fn sigfillset(set: *mut sigset_t) -> c_int;
}

// ============================================================================
// 时间结构 (time) — 尚未有 rusl-time crate, 定义本地占位
// ============================================================================

/// POSIX 时间规格 (repr(C))
#[repr(C)]
pub struct timespec {
    pub tv_sec: isize,
    pub tv_nsec: isize,
}

pub const CLOCK_MONOTONIC: c_int = 1;

extern "C" {
    pub fn clock_gettime(clk: c_int, ts: *mut timespec) -> c_int;
}

// ============================================================================
// futex 操作码常量 (与 Linux futex.h 一致, 在 rusl-internal 中亦有定义)
// ============================================================================

pub const FUTEX_WAIT: c_int = 128;
pub const FUTEX_WAKE: c_int = 129;
pub const FUTEX_PRIVATE: c_int = 128;

// __wake / __futexwait — musl static inline 函数, 不导出, 在 Rust 侧重新实现
// 参考 musl: src/internal/pthread_impl.h:168-180

pub const SYS_futex: i64 = 202;

pub unsafe fn __wake(addr: *const c_int, cnt: c_int) {
    let priv_: c_int = 1; // musl aio 始终使用 FUTEX_PRIVATE
    let cnt = if cnt < 0 { i32::MAX } else { cnt };
    let r1 = rusl_syscall::do_syscall!(SYS_futex, addr as i64, (FUTEX_WAKE | priv_) as i64, cnt as i64);
    if r1 == -22 {
        let _ = rusl_syscall::do_syscall!(SYS_futex, addr as i64, FUTEX_WAKE as i64, cnt as i64);
    }
}

pub unsafe fn __futexwait(addr: *const c_int, val: c_int, priv_: c_int) {
    let priv_ = if priv_ != 0 { FUTEX_PRIVATE } else { 0 };
    let r1 = rusl_syscall::do_syscall!(SYS_futex, addr as i64, (FUTEX_WAIT | priv_) as i64, val as i64, 0i64);
    if r1 == -22 {
        let _ = rusl_syscall::do_syscall!(SYS_futex, addr as i64, FUTEX_WAIT as i64, val as i64, 0i64);
    }
}

// __wait / __timedwait_cp — musl hidden 导出函数, 可通过 extern "C" 链接
extern "C" {
    pub fn __wait(addr: *const c_int, fut: *const c_int, val: c_int, priv_: c_int);
    pub fn __timedwait_cp(
        addr: *const c_int,
        val: c_int,
        clk: c_int,
        at: *const timespec,
        priv_: c_int,
    ) -> c_int;
}

// ============================================================================
// __getauxval / AT_MINSIGSTKSZ / MINSIGSTKSZ / PAGE_SIZE
// ============================================================================

// AT_MINSIGSTKSZ 辅助向量键
pub const AT_MINSIGSTKSZ: c_ulong = 51;
pub const MINSIGSTKSZ: usize = 2048;
pub const PAGE_SIZE: usize = 4096;

// fcntl 接口 (尚未在 rusl-unistd 中)
extern "C" {
    pub fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
}

// __getauxval — Linux 辅助向量查询 (musl 内部)
extern "C" {
    pub fn __getauxval(ty: c_ulong) -> c_ulong;
}

// ============================================================================
// 系统调用编号
// ============================================================================

#[cfg(target_arch = "x86_64")]
pub const SYS_rt_sigqueueinfo: i64 = 129;

// ============================================================================
// errno 常量 (本地定义 — rusl-errno 尚未提供完整常量集)
// ============================================================================

pub const EINVAL: c_int = 22;
pub const EAGAIN: c_int = 11;
pub const EBADF: c_int = 9;
pub const EINPROGRESS: c_int = 115;
pub const ECANCELED: c_int = 125;
pub const EIO: c_int = 5;
pub const EINTR: c_int = 4;
pub const ETIMEDOUT: c_int = 110;

/// errno 设置辅助函数
pub fn set_errno(e: c_int) {
    unsafe { *__errno_location() = e; }
}

// ============================================================================
// O_SYNC / O_DSYNC / O_APPEND / SEEK_CUR / F_GETFD / F_GETFL 等常量
// ============================================================================

pub const O_SYNC: c_int = 0x101000;
pub const O_DSYNC: c_int = 0x1000;
pub const O_APPEND: c_int = 0x400;
pub const SEEK_CUR: c_int = 1;
pub const F_GETFD: c_int = 1;
pub const F_GETFL: c_int = 3;
