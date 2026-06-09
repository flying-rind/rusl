//! 声明所有依赖其他模块的接口
//!
//! 当不开启rusl feature时，使用musl的C接口


#[cfg(not(feature = "rusl"))]
mod string {
    use core::ffi::{c_int, c_void};
    extern "C" {
        #[link_name = "memset"]
        fn musl_memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    }

    pub extern "C" fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void          { unsafe { musl_memset(s, c, n) } }

}

#[cfg(not(feature = "rusl"))]
mod internal {
    pub type size_t    = usize;

    // TLS module tracking
    #[repr(C)]
    pub struct tls_module {
        pub next: *mut tls_module,
        pub image: *mut core::ffi::c_void,
        pub len: size_t,
        pub size: size_t,
        pub align: size_t,
        pub offset: size_t,
    }

    // Locale map (placeholder — real definition in stage 9)
    #[repr(C)]
    pub struct __locale_map {
        _opaque: [u8; 64],  // placeholder size
    }
    // Locale struct (matches musl's struct __locale_struct)
    #[repr(C)]
    pub struct __locale_struct {
        pub cat: [*const __locale_map; 6],
    }


    #[repr(C)]
    pub struct __libc {
        pub can_do_threads: core::ffi::c_char,
        pub threaded: core::ffi::c_char,
        pub secure: core::ffi::c_char,
        pub need_locks: i8,            // volatile signed char — negative = need locks
        pub threads_minus_1: core::ffi::c_int,
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
    }
    pub use rusl_syscall::do_syscall;
}

#[cfg(not(feature = "rusl"))]
mod errno {
    use core::ffi::c_int;
    extern "C" {
        #[link_name = "__errno_location"]
        fn musl_errno_location() -> *mut c_int;
    }

    pub extern "C" fn __errno_location() -> *mut c_int { unsafe { musl_errno_location() } }
}

#[cfg(not(feature = "rusl"))]
pub use crate::import::{string::memset,  errno::__errno_location, internal::{do_syscall, __libc}};

#[cfg(feature = "rusl")]
pub use rusl_internal::do_syscall;
#[cfg(feature = "rusl")]
pub use rusl_errno::__errno_location;
#[cfg(feature = "rusl")]
pub use rusl_string::memset;
#[cfg(feature = "rusl")]
pub use rusl_internal::libc::__libc;
