//! # rusl-malloc
//!
//! `#![no_std]` Rust 实现的 musl libc malloc 模块。

#![no_std]
#![allow(non_camel_case_types)]
#![feature(custom_test_frameworks)]
#![test_runner(rusl_core::runner)]
#![reexport_test_harness_main = "test_main"]
#![no_main]

extern crate rusl_core;

pub mod allocator;
pub(crate) mod import;

// do_syscall! 在 crate 根可用 (根据 rusl feature 选择来源)
pub use crate::import::do_syscall;

#[path = "malloc_inner.rs"]
pub(crate) mod malloc_inner;

pub use malloc_inner::*;

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start(_argc: i32, _argv: *const *const u8) -> i32 {
    test_main();
    0
}
