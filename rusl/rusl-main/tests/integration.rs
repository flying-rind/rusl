//! 集成测试 — musl libc 对外导出接口
//!
//! ## 设计
//!
//! 使用 `rusl::framework` 自建 `no_std` 测试框架:
//! - `test!` 宏定义测试用例
//! - setjmp/longjmp 捕获 panic,每个测试独立运行
//! - Linux x86_64 sys_write/sys_exit 直接输出/退出
//!
//! ```bash
//! cargo test                         # 测试 Rust 实现
//! cargo test --features c-test       # 测试 musl libc C 实现
//! ```

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(test_framework::runner)]
#![reexport_test_harness_main = "test_main"]

#[cfg(feature = "rusl")]
extern crate alloc;
#[cfg(feature = "rusl")]
extern crate rusl_malloc;


/// 测试的ELF程序入口
#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    test_main();
    0
}

// 框架自身测试
#[path = "framework/mod.rs"]
mod framework;

// 各子 crate 集成测试
#[path = "string/mod.rs"]
mod string;

#[path = "env/mod.rs"]
mod env;

#[path = "ctype/mod.rs"]
mod ctype;

#[path = "stdlib/mod.rs"]
mod stdlib;

#[path = "prng/mod.rs"]
mod prng;

#[path = "search/mod.rs"]
mod search;

#[path = "regex/mod.rs"]
mod regex;

#[path = "exit/mod.rs"]
mod exit;

#[path = "unistd/mod.rs"]
mod unistd;

#[path = "stdio/mod.rs"]
mod stdio;

