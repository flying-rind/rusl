//! 声明所有依赖其他模块的接口
//!
//! 当不开启rusl feature时，使用musl的C接口

// ============================================================================
// rusl feature 开启时：直接 re-export rusl crate 符号
// ============================================================================

#[cfg(feature = "rusl")]
pub use rusl_internal::{atomic, libc, pthread_impl, defsysinfo};
#[cfg(feature = "rusl")]
pub use rusl_malloc::free::free;
#[cfg(feature = "rusl")]
pub use rusl_string::strchrnul;

// ============================================================================
// rusl feature 关闭时：提供 extern "C" / 手动定义 fallback
// ============================================================================

#[cfg(not(feature = "rusl"))]
pub mod atomic {
    extern "C" {
        #[link_name = "a_crash"]
        fn musl_a_crash() -> !;
    }

    pub fn a_crash() -> ! {
        unsafe { musl_a_crash() }
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
    use core::sync::atomic::{AtomicI32, AtomicU8, Ordering};

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
        #[link_name = "__pthread_self"]
        fn musl_pthread_self() -> pthread_t;
        #[link_name = "__set_thread_area"]
        fn musl_set_thread_area(p: *mut c_void) -> c_int;
    }

    pub fn __pthread_self() -> pthread_t {
        unsafe { musl_pthread_self() }
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
pub use malloc_ffi::free;
#[cfg(not(feature = "rusl"))]
pub use string_ffi::strchrnul;
