//! 声明所有依赖其他模块的接口
//!
//! 当不开启rusl feature时，使用musl的C接口

// __locale_struct 和相关类型
#[cfg(not(feature = "rusl"))]
mod types {
    #[repr(C)]
    pub struct __locale_map {
        _opaque: [u8; 64],
    }

    #[repr(C)]
    pub struct __locale_struct {
        pub cat: [*const __locale_map; 6],
    }
}

#[cfg(not(feature = "rusl"))]
pub use crate::import::types::__locale_struct;

#[cfg(feature = "rusl")]
pub use rusl_internal::libc::__locale_struct;
