//! # rusl
//!
//! `#![no_std]` Rust 实现的 musl libc 兼容库 — 聚合 crate。
//! 聚合所有子 crate 的符号导出, 提供统一的 staticlib 产出和集成测试入口。
//!
//! ## 子 crate
//!
//! - `rusl-core` — 共享基础设施 (c_types, framework, syscall, errno)
//! - `rusl-string` — 字符串/内存操作
//! - `rusl-stdlib` — 标准库工具函数
//! - `rusl-ctype` — 字符分类
//! - `rusl-malloc` — 内存分配器
//! - `rusl-regex` — 正则表达式/通配符匹配
//! - `rusl-prng` — 伪随机数
//! - `rusl-search` — 搜索/哈希表/二叉树
//! - `rusl-internal` — 内部基础设施
//! - `rusl-env` — 环境变量
//! - `rusl-unistd` — POSIX 系统调用封装
//! - `rusl-stdio` — 标准 I/O
//! - `rusl-exit` — 进程终止

#![no_std]
#![allow(non_camel_case_types)]
#![feature(custom_test_frameworks)]
#![test_runner(test_framework::runner)]
#![reexport_test_harness_main = "test_main"]
#![no_main]

pub mod api;

#[cfg(not(feature = "rusl"))]
use core::sync::atomic::{AtomicPtr, Ordering};
#[cfg(not(feature = "rusl"))]
use core::panic::PanicInfo;
#[cfg(not(feature = "rusl"))]
use core::alloc::{GlobalAlloc, Layout};

extern crate rusl_core;
#[cfg(feature = "rusl")]
extern crate rusl_malloc;
#[cfg(feature = "rusl")]
extern crate alloc;

#[cfg(feature = "rusl")]
// errno
pub use rusl_core::errno::{__errno_location, ___errno_location, set_errno, EINVAL};
// syscall
#[cfg(feature = "rusl")]
pub use rusl_core::syscall::*;

#[cfg(feature = "rusl")]
pub use rusl_string::*;
#[cfg(feature = "rusl")]
pub use rusl_stdlib::*;
#[cfg(feature = "rusl")]
pub use rusl_malloc::*;
#[cfg(feature = "rusl")]
pub use rusl_regex::*;
#[cfg(feature = "rusl")]
pub use rusl_prng::*;
#[cfg(feature = "rusl")]
pub use rusl_search::*;
#[cfg(feature = "rusl")]
pub use rusl_env::*;
#[cfg(feature = "rusl")]
pub use rusl_unistd::*;
#[cfg(feature = "rusl")]
pub use rusl_stdio::*;
#[cfg(feature = "rusl")]
pub use rusl_exit::*;
// ctype 必须最后 re-export (locale_t 在 c_types 和 ctype 中都有)
#[cfg(feature = "rusl")]
pub use rusl_ctype::*;
#[cfg(not(feature = "rusl"))]
pub use api::ctype::*;

#[cfg(not(feature = "rusl"))]
static PANIC_HOOK: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());

#[cfg(not(feature = "rusl"))]
#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    let hook = PANIC_HOOK.load(Ordering::SeqCst);
    if !hook.is_null() {
        unsafe {
            let hook_fn: unsafe extern "C" fn(*const PanicInfo) -> ! =
                core::mem::transmute(hook);
            hook_fn(info as *const PanicInfo);
        }
    }
    loop {}
}

#[cfg(not(feature = "rusl"))]

#[no_mangle]
pub unsafe extern "C" fn __rusl_set_panic_hook(hook: unsafe extern "C" fn(*const PanicInfo) -> !) {
    PANIC_HOOK.store(hook as *mut (), Ordering::SeqCst);
}

#[cfg(not(feature = "rusl"))]
struct RuslCAlloc;

#[cfg(not(feature = "rusl"))]
unsafe impl Sync for RuslCAlloc {}

#[cfg(not(feature = "rusl"))]
unsafe impl GlobalAlloc for RuslCAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = if layout.size() == 0 { 1 } else { layout.size() };
        crate::api::malloc::malloc(size) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        crate::api::malloc::free::free(ptr as *mut core::ffi::c_void);
    }

    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        let size = if new_size == 0 { 1 } else { new_size };
        crate::api::malloc::realloc::realloc(ptr as *mut core::ffi::c_void, size) as *mut u8
    }
}

#[cfg(not(feature = "rusl"))]
#[global_allocator]
static GLOBAL: RuslCAlloc = RuslCAlloc;

// ---- 测试入口 ----
#[cfg(test)]
#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    test_main();
    0
}

