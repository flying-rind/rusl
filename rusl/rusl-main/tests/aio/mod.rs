//! aio 集成测试子模块

#[allow(unused)]
pub use rusl::api::aio::*;
#[allow(unused)]
pub use rusl::api::unistd::*;
#[allow(unused)]
pub use core::ffi::c_int;
#[allow(unused)]
pub use core::ffi::c_void;

// errno 常量 (Linux x86_64)
pub const EINPROGRESS: c_int = 115;
pub const EBADF: c_int = 9;

// fcntl 常量
pub const O_SYNC: c_int = 0x101000;

// timespec 布局兼容结构体 (用于 aio_suspend)
#[repr(C)]
pub struct Timespec {
    pub tv_sec: isize,
    pub tv_nsec: isize,
}

mod aio_tests;
mod aio_suspend_tests;
mod lio_listio_tests;
