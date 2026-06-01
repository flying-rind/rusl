//! # rusl-prng
//!
//! `#![no_std]` Rust 实现的 musl libc prng 模块。

#![no_std]
#![allow(non_camel_case_types)]
#![feature(custom_test_frameworks)]
#![test_runner(rusl_core::runner)]
#![reexport_test_harness_main = "test_main"]
#![no_main]

extern crate rusl_core;


#[path = "prng_inner.rs"]
pub(crate) mod prng_inner;

pub use prng_inner::*;

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start(_argc: i32, _argv: *const *const u8) -> i32 {
    test_main();
    0
}
