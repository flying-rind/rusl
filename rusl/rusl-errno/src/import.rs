//! 声明所有依赖其他模块的接口
//!
//! 当不开启rusl feature时，使用musl的C接口

// __locale_struct 和相关类型
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

pub use crate::import::types::__locale_struct;
