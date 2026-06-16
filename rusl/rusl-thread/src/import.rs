//! # 外部依赖导入模块
//!
//! 当开启 `rusl` feature 时，依赖其他 rusl-xxx crate；
//! 否则使用 `extern "C"` 链接 musl libc 的 C 实现。

#![allow(dead_code, unused_imports, unused_variables)]

use core::ffi::{c_char, c_int, c_void};

// ============================================================================
// 内存分配接口
// ============================================================================

#[cfg(feature = "rusl")]
pub use rusl_malloc::{malloc, calloc, free};

#[cfg(not(feature = "rusl"))]
pub use crate::import::stdlib::{malloc, calloc, free, __libc_malloc};

#[cfg(not(feature = "rusl"))]
mod stdlib {
    use core::ffi::c_void;
    extern "C" {
        #[link_name = "malloc"]
        fn musl_malloc(size: usize) -> *mut c_void;
        #[link_name = "calloc"]
        fn musl_calloc(nmemb: usize, size: usize) -> *mut c_void;
        #[link_name = "free"]
        fn musl_free(ptr: *mut c_void);
        #[link_name = "__libc_malloc"]
        pub fn __libc_malloc(size: usize) -> *mut c_void;
    }
    pub extern "C" fn malloc(size: usize) -> *mut c_void { unsafe { musl_malloc(size) } }
    pub extern "C" fn calloc(nmemb: usize, size: usize) -> *mut c_void { unsafe { musl_calloc(nmemb, size) } }
    pub extern "C" fn free(ptr: *mut c_void) { unsafe { musl_free(ptr) } }
}

// ============================================================================
// 文件 I/O 接口 — 通过 extern "C" 直接声明 (避免依赖特定 crate 导出)
// ============================================================================

extern "C" {
    #[link_name = "read"]
    fn musl_read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    #[link_name = "write"]
    fn musl_write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    #[link_name = "close"]
    fn musl_close(fd: c_int) -> c_int;
    #[link_name = "access"]
    fn musl_access(path: *const c_char, mode: c_int) -> c_int;
    #[link_name = "link"]
    fn musl_link(old: *const c_char, new: *const c_char) -> c_int;
    #[link_name = "unlink"]
    fn musl_unlink(path: *const c_char) -> c_int;
    #[link_name = "fstat"]
    fn musl_fstat(fd: c_int, buf: *mut c_void) -> c_int;
    #[link_name = "shm_unlink"]
    pub fn shm_unlink(name: *const c_char) -> c_int;
}

pub extern "C" fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize { unsafe { musl_read(fd, buf, count) } }
pub extern "C" fn write(fd: c_int, buf: *const c_void, count: usize) -> isize { unsafe { musl_write(fd, buf, count) } }
pub extern "C" fn close(fd: c_int) -> c_int { unsafe { musl_close(fd) } }
pub extern "C" fn access(path: *const c_char, mode: c_int) -> c_int { unsafe { musl_access(path, mode) } }
pub extern "C" fn link(old: *const c_char, new: *const c_char) -> c_int { unsafe { musl_link(old, new) } }
pub extern "C" fn unlink(path: *const c_char) -> c_int { unsafe { musl_unlink(path) } }
pub extern "C" fn fstat(fd: c_int, buf: *mut c_void) -> c_int { unsafe { musl_fstat(fd, buf) } }

// open 是可变参数函数，通过 extern "C" 声明
extern "C" {
    #[link_name = "open"]
    fn musl_open(path: *const c_char, flags: c_int, ...) -> c_int;
}
pub unsafe extern "C" fn open(path: *const c_char, flags: c_int, _: ...) -> c_int { unimplemented!() }

// ============================================================================
// 字符串操作接口
// ============================================================================

#[cfg(feature = "rusl")]
pub use rusl_string::{strnlen, memcmp, memset, memcpy};

#[cfg(not(feature = "rusl"))]
pub use crate::import::string::{strnlen, memcmp, memset, memcpy};

#[cfg(not(feature = "rusl"))]
mod string {
    use core::ffi::{c_char, c_int, c_void};
    extern "C" {
        #[link_name = "strnlen"]
        fn musl_strnlen(s: *const c_char, maxlen: usize) -> usize;
        #[link_name = "memcmp"]
        fn musl_memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
        #[link_name = "memset"]
        fn musl_memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
        #[link_name = "memcpy"]
        fn musl_memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    }
    pub extern "C" fn strnlen(s: *const c_char, maxlen: usize) -> usize { unsafe { musl_strnlen(s, maxlen) } }
    pub extern "C" fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int { unsafe { musl_memcmp(a, b, n) } }
    pub extern "C" fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void { unsafe { musl_memset(s, c, n) } }
    pub extern "C" fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void { unsafe { musl_memcpy(dst, src, n) } }
}

// ============================================================================
// 格式化输出接口 — 通过 extern "C" 直接声明
// ============================================================================

extern "C" {
    #[link_name = "snprintf"]
    fn musl_snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> i32;
}
// snprintf 是可变参数函数，必须以 unsafe 导出
pub unsafe extern "C" fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, _: ...) -> i32 { unimplemented!() }

// ============================================================================
// 时钟接口
// ============================================================================

extern "C" {
    #[link_name = "clock_gettime"]
    pub fn clock_gettime(clk: crate::types::clockid_t, ts: *mut crate::types::timespec) -> c_int;
    #[link_name = "__clock_nanosleep"]
    pub fn __clock_nanosleep(
        clk: crate::types::clockid_t,
        flags: c_int,
        req: *const crate::types::timespec,
        rem: *mut crate::types::timespec,
    ) -> c_int;
}

// ============================================================================
// errno 接口
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

/// errno 设置辅助函数
pub fn set_errno(e: c_int) {
    unsafe { *__errno_location() = e; }
}

// ============================================================================
// 系统调用接口 (来自 rusl-syscall, 始终可用)
// ============================================================================

pub use rusl_syscall::do_syscall;

// ============================================================================
// 内部接口 (来自 rusl-internal)
// ============================================================================

#[cfg(feature = "rusl")]
pub use rusl_internal::libc::__libc;

#[cfg(not(feature = "rusl"))]
pub use crate::import::internal::__libc;

#[cfg(not(feature = "rusl"))]
mod internal {
    use core::ffi;
    pub type size_t = usize;

    #[repr(C)]
    pub struct tls_module {
        pub next: *mut tls_module,
        pub image: *mut ffi::c_void,
        pub len: size_t,
        pub size: size_t,
        pub align: size_t,
        pub offset: size_t,
    }

    #[repr(C)]
    pub struct __locale_map {
        _opaque: [u8; 64],
    }

    #[repr(C)]
    pub struct __locale_struct {
        pub cat: [*const __locale_map; 6],
    }

    #[repr(C)]
    pub struct __libc {
        pub can_do_threads: ffi::c_char,
        pub threaded: ffi::c_char,
        pub secure: ffi::c_char,
        pub need_locks: i8,
        pub threads_minus_1: ffi::c_int,
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
}
