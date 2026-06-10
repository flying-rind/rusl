//! 声明所有依赖其他模块的接口
//!
//! 当不开启rusl feature时，使用musl的C接口
//!
//! 此处声明的 extern "C" 接口来自 musl 的非 internal 模块
//! （如 stdio/ctype），这些符号在最终链接时由 musl 提供。

#[cfg(not(feature = "rusl"))]
mod errno {
    use core::ffi::c_int;
    extern "C" {
        #[link_name = "__errno_location"]
        fn musl_errno_location() -> *mut c_int;
    }

    pub extern "C" fn __errno_location() -> *mut c_int {
        unsafe { musl_errno_location() }
    }
}

/// stdio 子系统提供的接口（musl src/stdio/）
#[cfg(not(feature = "rusl"))]
mod stdio {
    use crate::file::FILE;
    use core::ffi::c_int;

    extern "C" {
        /// 从 FILE 流中获取下一个字符（必要时填充缓冲区）
        /// 定义: musl src/stdio/__uflow.c
        pub fn __uflow(f: *mut FILE) -> c_int;

        /// 将 FILE 切换到读模式
        /// 定义: musl src/stdio/__toread.c
        pub fn __toread(f: *mut FILE) -> c_int;
    }
}

/// floatscan 辅助函数（由 musl floatscan_wrap.c 提供 80-bit 中间精度）
#[cfg(not(feature = "rusl"))]
mod floatscan_helpers {
    extern "C" {
        pub fn __floatscan_scale(y: f64, e2: i32) -> f64;
        pub fn __floatscan_mul(a: f64, b: f64) -> f64;
        pub fn __floatscan_abs(x: f64) -> f64;
        pub fn __floatscan_copysign(x: f64, y: f64) -> f64;
        pub fn __floatscan_fmod(x: f64, y: f64) -> f64;
    }
}

#[cfg(not(feature = "rusl"))]
pub use crate::import::errno::__errno_location;
#[cfg(not(feature = "rusl"))]
pub use crate::import::stdio::{__uflow, __toread};
#[cfg(not(feature = "rusl"))]
pub use crate::import::floatscan_helpers::{
    __floatscan_scale, __floatscan_mul, __floatscan_abs,
    __floatscan_copysign, __floatscan_fmod,
};

#[cfg(feature = "rusl")]
pub use rusl_errno::__errno_location;
