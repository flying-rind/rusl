//! # rusl-search
//!
//! `#![no_std]` Rust 实现的 musl libc search 模块。

#![no_std]
#![allow(non_camel_case_types)]
#![feature(custom_test_frameworks)]
#![test_runner(rusl_core::runner)]
#![reexport_test_harness_main = "test_main"]
#![no_main]

extern crate rusl_core;
extern crate rusl_malloc;
extern crate alloc;

#[path = "search_inner.rs"]
mod search_inner;

pub use search_inner::*;

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start(_argc: i32, _argv: *const *const u8) -> i32 {
    test_main();
    0
}
