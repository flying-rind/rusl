//! 条件导入层 — 根据 feature 切换依赖来源。
//!
//! - `rusl` feature 开启时：使用其他 rusl crate 的 Rust 实现
//! - `rusl` feature 关闭时：通过 `extern "C"` 调用 musl libc

pub use imp::__errno_location;

#[cfg(feature = "rusl")]
mod imp {
    pub use rusl_errno::__errno_location;
}

#[cfg(not(feature = "rusl"))]
mod imp {
    use core::ffi::c_int;

    extern "C" {
        #[link_name = "__errno_location"]
        fn musl___errno_location() -> *mut c_int;
    }

    /// 非 rusl 模式下，直接调用 musl libc 的 `__errno_location`。
    pub unsafe fn __errno_location() -> *mut c_int {
        unsafe { musl___errno_location() }
    }
}
