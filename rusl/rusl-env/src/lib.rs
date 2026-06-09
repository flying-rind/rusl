//! # rusl-env
//!
//! `#![no_std]` Rust 实现的 musl libc env 模块。

#![no_std]
#![allow(non_camel_case_types)]
#![feature(custom_test_frameworks)]
#![feature(linkage)]
#![test_runner(rusl_core::runner)]
#![reexport_test_harness_main = "test_main"]
#![no_main]

extern crate rusl_core;
extern crate rusl_syscall;
#[cfg(feature = "rusl")]
extern crate rusl_malloc;
extern crate alloc;

pub(crate) mod import;

#[path = "env_inner.rs"]
pub(crate) mod env_inner;

pub use env_inner::*;

// 不开启rusl时与musl libc链接，需要提供global allocator
#[cfg(not(feature = "rusl"))]
mod allocator;

#[cfg(test)]
#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    test_main();
    0
}
