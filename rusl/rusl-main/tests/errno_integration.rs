//! 集成测试 — rusl_errno 对外导出接口
//!
//! 测试 musl libc 中 errno 相关的对外 API:
//! - `__errno_location` / `___errno_location` — errno 位置访问器
//! - `strerror` / `strerror_l` — 错误码到消息字符串映射

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(test_framework::runner)]
#![reexport_test_harness_main = "test_main"]

#[cfg(feature = "rusl")]
extern crate rusl_malloc;

// rusl 库 crate 的 panic_handler 必须通过 extern crate 拉入测试二进制。
// 在 no_std + --test + -Z panic-abort-tests 模式下, 测试二进制作为根 crate
// 需要 panic_handler; 该符号定义于 rusl-main/src/lib.rs (cfg(not(feature="rusl"))).
extern crate rusl;

use test_framework::test;

#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    test_main();
    0
}

test!("framework_ok" {
    assert!(true);
});

#[path = "errno/mod.rs"]
mod errno;