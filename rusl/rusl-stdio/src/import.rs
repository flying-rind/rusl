//! 声明所有依赖其他模块的接口
//!
//! 当不开启rusl feature时，使用musl的C接口

#[cfg(not(feature = "rusl"))]
mod string {
    use core::ffi::c_char;
    extern "C" {
        #[link_name = "strnlen"]
        fn musl_strnlen(s: *const c_char, n: usize) -> usize;
    }

    pub extern "C" fn strnlen(s: *const c_char, n: usize) -> usize {
        unsafe { musl_strnlen(s, n) }
    }
}

#[cfg(not(feature = "rusl"))]
pub use crate::import::string::strnlen;

#[cfg(feature = "rusl")]
pub use rusl_string::strnlen;
