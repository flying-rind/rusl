//! # rusl-core
//!
//! `#![no_std]` 共享基础设施: C 类型定义, 测试框架, 系统调用封装, errno 处理。
//! 应当包含musl/src/internal/syscall.h, musl/src/errno/
//!
//! ```

#![no_std]
#![allow(non_camel_case_types)]
#![feature(custom_test_frameworks)]
#![test_runner(runner)]
#![reexport_test_harness_main = "test_main"]
#![no_main]

// ---------------------------------------------------------------------------
// panic_handler
// ---------------------------------------------------------------------------
#[cfg(feature = "rusl")]
use core::panic::PanicInfo;
#[cfg(feature = "rusl")]
use core::sync::atomic::{AtomicPtr, Ordering};

#[cfg(feature = "rusl")]
static PANIC_HOOK: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());

#[cfg(feature = "rusl")]
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

#[cfg(feature = "rusl")]
#[no_mangle]
pub fn __rusl_set_panic_hook(hook: fn(*const PanicInfo) -> !) {
    PANIC_HOOK.store(hook as *mut (), Ordering::SeqCst);
}

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start(_argc: i32, _argv: *const *const u8) -> i32 {
    test_main();
    0
}


// ---------------------------------------------------------------------------
// 公共模块
// ---------------------------------------------------------------------------

pub mod c_types;
pub mod arch;
pub mod syscall;
pub mod errno;

pub use c_types::*;
pub use arch::*;
pub use syscall::*;


// ---------------------------------------------------------------------------
// 宏重新导出自 test-framework (使 rusl_core::test! 等路径继续有效)
// ---------------------------------------------------------------------------

pub use test_framework::test;
pub use test_framework::print;
pub use test_framework::println;

// ---------------------------------------------------------------------------
// 宏重新导出 (供依赖 rusl-core 的 crate 使用)
// ---------------------------------------------------------------------------

pub use test_framework::{run_test, runner, test_panic_handler, install_panic_hook};