//! # rusl-stdio
//!
//! `#![no_std]` Rust 实现的 musl libc stdio 模块。

#![no_std]
#![allow(non_camel_case_types)]
#![feature(c_variadic)]
#![feature(custom_test_frameworks)]
#![test_runner(rusl_core::runner)]
#![reexport_test_harness_main = "test_main"]
#![no_main]

extern crate rusl_core;
#[cfg(test)]
extern crate rusl_malloc;
#[cfg(test)]
extern crate alloc;

pub(crate) mod import;

#[path = "stdio_inner.rs"]
pub(crate) mod stdio_inner;


pub use stdio_inner::*;

// 导出内部类型, 使外部可命名 FILE 和 VaList
pub use stdio_inner::stdio_impl::{FILE, VaList};

// 不开启rusl时与musl libc链接，需要提供global allocator
#[cfg(not(feature = "rusl"))]
mod allocator;

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start(_argc: i32, _argv: *const *const u8) -> i32 {
    test_main();
    0
}
