//! 声明所有依赖其他模块的接口
//!
//! 当开启 rusl feature 时，从其他 rusl-xx crate 导入
//! 否则使用 extern "C" 或本地定义 fallback

// ============================================================================
// rusl feature 开启时：直接 re-export rusl crate 符号
// ============================================================================

#[cfg(feature = "rusl")]
pub use rusl_internal::{atomic, libc, pthread_impl, defsysinfo};
#[cfg(feature = "rusl")]
pub use rusl_malloc::free::free;
#[cfg(feature = "rusl")]
pub use rusl_string::strchrnul;
#[cfg(feature = "rusl")]
pub use rusl_errno::{__errno_location, set_errno, EINVAL};
#[cfg(feature = "rusl")]
pub use rusl_internal::syscall;

// ============================================================================
// rusl feature 关闭时：提供 extern "C" / 手动定义 fallback
// ============================================================================

#[cfg(not(feature = "rusl"))]
pub mod atomic {
    /// 触发崩溃 —— 对应 musl `atomic.h` 中的 `static inline void a_crash()`。
    /// musl 中 a_crash 是内联函数，不作为外部 C 符号导出，
    /// 因此 rusl-env 必须自行实现。
    pub fn a_crash() -> ! {
        // 写入空指针触发 SIGSEGV，与 musl 行为一致
        unsafe {
            core::ptr::write_volatile(core::ptr::null_mut::<u8>(), 0);
        }
        // 如果上面的写入没有触发信号，则进入无限循环
        loop {
            core::hint::spin_loop();
        }
    }
}

#[cfg(not(feature = "rusl"))]
pub mod libc {
    use core::ffi::c_int;
    use rusl_core::c_types::size_t;

    #[repr(C)]
    pub struct __locale_map {
        _opaque: [u8; 64],
    }

    #[repr(C)]
    pub struct __locale_struct {
        pub cat: [*const __locale_map; 6],
    }

    #[repr(C)]
    pub struct tls_module {
        pub next: *mut tls_module,
        pub image: *mut core::ffi::c_void,
        pub len: size_t,
        pub size: size_t,
        pub align: size_t,
        pub offset: size_t,
    }

    #[repr(C)]
    pub struct __libc {
        pub can_do_threads: core::ffi::c_char,
        pub threaded: core::ffi::c_char,
        pub secure: core::ffi::c_char,
        pub need_locks: i8,
        pub threads_minus_1: c_int,
        pub auxv: *mut size_t,
        pub tls_head: *mut tls_module,
        pub tls_size: size_t,
        pub tls_align: size_t,
        pub tls_cnt: size_t,
        pub page_size: size_t,
        pub global_locale: __locale_struct,
    }

    extern "C" {
        pub static mut __libc: __libc;
        pub static mut __hwcap: size_t;
        pub static mut __progname: *const core::ffi::c_char;
        pub static mut __progname_full: *const core::ffi::c_char;
    }
}

#[cfg(not(feature = "rusl"))]
pub mod pthread_impl {
    use core::ffi::{c_int, c_uint, c_void};
    use core::sync::atomic::{AtomicI32, AtomicU8};

    pub type pthread_t = *mut Pthread;

    #[repr(C)]
    pub struct Ptcb {
        _private: [u8; 32],
    }

    #[repr(C)]
    pub struct RobustList {
        pub head: *mut c_void,
        pub off: usize,
        pub pending: *mut c_void,
    }

    #[repr(C)]
    pub struct SpinLock {
        _private: [u8; 4],
    }

    #[repr(C)]
    pub struct Locale {
        _private: [u8; 64],
    }

    #[repr(C)]
    pub enum DetachState {
        Exited = 0,
        Exiting = 1,
        Joinable = 2,
    }

    #[cfg(target_arch = "x86_64")]
    pub const TLS_ABOVE_TP: bool = false;
    #[cfg(target_arch = "aarch64")]
    pub const TLS_ABOVE_TP: bool = true;

    #[repr(C)]
    pub struct Pthread {
        pub self_: *mut Pthread,
        #[cfg(not(TLS_ABOVE_TP))]
        pub dtv: *mut usize,
        pub prev: *mut Pthread,
        pub next: *mut Pthread,
        pub sysinfo: usize,
        #[cfg(not(TLS_ABOVE_TP))]
        pub canary: usize,
        pub tid: c_int,
        pub errno_val: c_int,
        pub detach_state: AtomicI32,
        pub cancel: AtomicI32,
        pub canceldisable: AtomicU8,
        pub cancelasync: AtomicU8,
        pub tsd_used: bool,
        pub dlerror_flag: bool,
        pub map_base: *mut u8,
        pub map_size: usize,
        pub stack: *mut c_void,
        pub stack_size: usize,
        pub guard_size: usize,
        pub result: *mut c_void,
        pub cancelbuf: *mut Ptcb,
        pub tsd: *mut *mut c_void,
        pub robust_list: RobustList,
        pub h_errno_val: c_int,
        pub timer_id: AtomicI32,
        pub locale: Locale,
        pub killlock: SpinLock,
        pub dlerror_buf: *mut u8,
        pub stdio_locks: *mut c_void,
        #[cfg(TLS_ABOVE_TP)]
        pub canary: usize,
        #[cfg(TLS_ABOVE_TP)]
        pub dtv: *mut usize,
    }

    pub const DEFAULT_STACK_MAX: usize = 8 << 20;
    pub const DEFAULT_STACK_SIZE: usize = 80 * 1024;
    pub static mut DEFAULT_STACKSIZE: c_uint = DEFAULT_STACK_SIZE as c_uint;

    extern "C" {
        #[link_name = "__set_thread_area"]
        fn musl_set_thread_area(p: *mut c_void) -> c_int;
    }

    /// 获取当前线程的 Pthread 指针。
    /// 对应 musl `pthread_impl.h` 中的内联宏：
    /// `#define __pthread_self() ((pthread_t)(__get_tp() - sizeof(struct __pthread) - TP_OFFSET))`
    /// musl 中 __pthread_self 是内联宏，不作为外部 C 符号导出，
    /// 因此 rusl-env 必须用内联汇编自行实现。
    pub fn __pthread_self() -> pthread_t {
        let tp: usize;
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("mov {}, fs:0", out(reg) tp, options(nostack, preserves_flags));
        }
        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("mrs {}, tpidr_el0", out(reg) tp, options(nostack, preserves_flags));
        }
        // TLS Below TP (x86_64): TCB 在 TP 之下
        #[cfg(not(TLS_ABOVE_TP))]
        { (tp - core::mem::size_of::<Pthread>()) as pthread_t }
        // TLS Above TP (aarch64): TCB 在 TP 之上
        #[cfg(TLS_ABOVE_TP)]
        { tp as pthread_t }
    }

    pub fn set_thread_area(p: *mut c_void) -> c_int {
        unsafe { musl_set_thread_area(p) }
    }

    pub static THREAD_LIST_LOCK: SpinLock = SpinLock { _private: [0u8; 4] };
}

#[cfg(not(feature = "rusl"))]
pub mod defsysinfo {
    use core::sync::atomic::AtomicUsize;
    pub static __SYSINFO: AtomicUsize = AtomicUsize::new(0);
}

// ---------------------------------------------------------------------------
// errno — 线程局部 errno 访问
// ---------------------------------------------------------------------------

#[cfg(not(feature = "rusl"))]
pub mod errno {
    use core::ffi::c_int;

    /// EINVAL — 参数无效 (Invalid argument)。
    /// POSIX.1-2001 定义。Linux x86_64 / aarch64 上 errno 值 = 22。
    pub const EINVAL: c_int = 22;

    extern "C" {
        #[link_name = "__errno_location"]
        fn musl_errno_location() -> *mut c_int;
    }

    /// 返回指向当前线程 errno 变量的指针。
    pub fn __errno_location() -> *mut c_int {
        unsafe { musl_errno_location() }
    }

    /// 设置当前线程的 errno 值。
    ///
    /// # Safety
    ///
    /// 调用者必须确保在适当的线程上下文中调用。
    pub unsafe fn set_errno(val: c_int) {
        unsafe { *__errno_location() = val; }
    }
}

#[cfg(not(feature = "rusl"))]
pub use errno::{__errno_location, set_errno, EINVAL};

// ---------------------------------------------------------------------------
// syscall — 原始系统调用
// ---------------------------------------------------------------------------

#[cfg(not(feature = "rusl"))]
pub mod syscall {
    use rusl_core::arch;

    // --- x86_64 syscall numbers ---
    #[cfg(target_arch = "x86_64")]
    pub const SYS_exit: i64 = 60;
    #[cfg(target_arch = "x86_64")]
    pub const SYS_exit_group: i64 = 231;
    #[cfg(target_arch = "x86_64")]
    pub const SYS_open: i64 = 2;
    #[cfg(target_arch = "x86_64")]
    pub const SYS_poll: i64 = 7;
    #[cfg(target_arch = "x86_64")]
    pub const SYS_ppoll: i64 = 271;
    #[cfg(target_arch = "x86_64")]
    pub const SYS_arch_prctl: i64 = 158;
    #[cfg(target_arch = "x86_64")]
    pub const SYS_set_tid_address: i64 = 218;
    #[cfg(target_arch = "x86_64")]
    pub const SYS_mmap: i64 = 9;

    // --- aarch64 syscall numbers ---
    #[cfg(target_arch = "aarch64")]
    pub const SYS_exit: i64 = 93;
    #[cfg(target_arch = "aarch64")]
    pub const SYS_exit_group: i64 = 94;
    #[cfg(target_arch = "aarch64")]
    pub const SYS_open: i64 = 1024; // openat on aarch64
    #[cfg(target_arch = "aarch64")]
    pub const SYS_ppoll: i64 = 73;
    #[cfg(target_arch = "aarch64")]
    pub const SYS_set_tid_address: i64 = 96;
    #[cfg(target_arch = "aarch64")]
    pub const SYS_mmap: i64 = 222;

    // --- raw syscall wrappers ---

    #[inline]
    pub unsafe fn raw_syscall1(nr: i64, a1: i64) -> i64 {
        arch::__syscall1(nr, a1)
    }

    #[inline]
    pub unsafe fn raw_syscall2(nr: i64, a1: i64, a2: i64) -> i64 {
        arch::__syscall2(nr, a1, a2)
    }

    #[inline]
    pub unsafe fn raw_syscall3(nr: i64, a1: i64, a2: i64, a3: i64) -> i64 {
        arch::__syscall3(nr, a1, a2, a3)
    }

    #[inline]
    pub unsafe fn raw_syscall5(nr: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> i64 {
        arch::__syscall5(nr, a1, a2, a3, a4, a5)
    }

    #[inline]
    pub unsafe fn raw_syscall6(nr: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64, a6: i64) -> i64 {
        arch::__syscall6(nr, a1, a2, a3, a4, a5, a6)
    }
}

// ---------------------------------------------------------------------------
// malloc — free
// ---------------------------------------------------------------------------

#[cfg(not(feature = "rusl"))]
mod malloc_ffi {
    use core::ffi::c_void;
    extern "C" {
        #[link_name = "free"]
        fn musl_free(p: *mut c_void);
    }
    pub fn free(p: *mut c_void) {
        unsafe { musl_free(p) }
    }
}

#[cfg(not(feature = "rusl"))]
pub use malloc_ffi::free;

// ---------------------------------------------------------------------------
// string — strchrnul
// ---------------------------------------------------------------------------

#[cfg(not(feature = "rusl"))]
mod string_ffi {
    use core::ffi::{c_char, c_int};
    extern "C" {
        #[link_name = "strchrnul"]
        fn musl_strchrnul(s: *const c_char, c: c_int) -> *mut c_char;
    }
    pub fn strchrnul(s: *const c_char, c: c_int) -> *mut c_char {
        unsafe { musl_strchrnul(s, c) }
    }
}

#[cfg(not(feature = "rusl"))]
pub use string_ffi::strchrnul;
