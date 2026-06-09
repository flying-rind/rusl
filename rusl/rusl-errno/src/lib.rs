//! # rusl-errno
//!
//! `#![no_std]` Rust 实现的 musl libc errno 模块。
//!
//! 包含:
//! - `__errno_location` / `___errno_location` — 线程局部 errno 访问
//! - `strerror` / `strerror_l` — 错误码到可读描述字符串的映射
//! - `strerror_r` / `__xpg_strerror_r` — 线程安全的错误消息缓冲区拷贝

#![no_std]
#![allow(non_camel_case_types)]
#![feature(custom_test_frameworks)]
#![test_runner(rusl_core::runner)]
#![reexport_test_harness_main = "test_main"]
#![no_main]

extern crate rusl_core;

pub(crate) mod import;

#[path = "errno_inner.rs"]
mod errno_inner;

pub use errno_inner::*;

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start(_argc: i32, _argv: *const *const u8) -> i32 {
    test_main();
    0
}
