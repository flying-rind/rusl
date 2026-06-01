//! # rusl-regex
//!
//! `#![no_std]` Rust 实现的 musl libc regex 模块。

#![no_std]
#![allow(non_camel_case_types)]
#![feature(custom_test_frameworks)]
#![test_runner(rusl_core::runner)]
#![reexport_test_harness_main = "test_main"]
#![no_main]

extern crate rusl_core;
extern crate rusl_malloc;
extern crate alloc;

#[path = "regex_inner.rs"]
pub(crate) mod regex_inner;


pub use regex_inner::*;

#[cfg(test)]
#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    test_main();
    0
}
