//! 集成测试 — rusl_ctype 对外导出接口

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(test_framework::runner)]
#![reexport_test_harness_main = "test_main"]

use test_framework::test;
#[cfg(feature = "rusl")]
extern crate rusl_malloc;


#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    test_main();
    0
}

test!("framework_ok" {
    assert!(true);
});

#[path = "ctype/mod.rs"]
mod ctype;
