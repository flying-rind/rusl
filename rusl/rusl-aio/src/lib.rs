//! # rusl-aio
//!
//! `#![no_std]` Rust 实现的 musl libc aio 模块。
//!
//! 提供 POSIX 异步 I/O (AIO) 接口:
//! - `aio_read`, `aio_write`, `aio_fsync` — 提交异步 I/O 请求
//! - `aio_return`, `aio_error` — 查询操作结果
//! - `aio_cancel` — 取消异步操作
//! - `aio_suspend` — 挂起等待异步操作完成
//! - `lio_listio` — 批量提交异步 I/O 请求列表

#![no_std]
#![allow(non_camel_case_types)]
#![feature(custom_test_frameworks)]
#![test_runner(rusl_core::runner)]
#![reexport_test_harness_main = "test_main"]
#![no_main]

extern crate rusl_core;

pub(crate) mod import;

mod aio;
mod aio_suspend;
mod lio_listio;

// 导出公共 API
pub use aio::{
    aiocb,
    AIO_CANCELED, AIO_NOTCANCELED, AIO_ALLDONE,
    LIO_READ, LIO_WRITE, LIO_NOP,
    LIO_WAIT, LIO_NOWAIT,
    aio_read, aio_write, aio_fsync,
    aio_return, aio_error, aio_cancel,
    __aio_close, __aio_atfork,
    __aio_fut,
};
pub use aio_suspend::aio_suspend;
pub use lio_listio::lio_listio;

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start(_argc: i32, _argv: *const *const u8) -> i32 {
    test_main();
    0
}
