//! 集成测试 — rusl_stdio 对外导出接口

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(test_framework::runner)]
#![reexport_test_harness_main = "test_main"]

use rusl_core::test;
#[cfg(feature = "rusl")]
extern crate rusl_malloc;
#[cfg(feature = "rusl")]
extern crate rusl_env;

#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    test_main();
    0
}

test!("framework_ok" {
    assert!(true);
});

#[path = "stdio/mod.rs"]
mod stdio;