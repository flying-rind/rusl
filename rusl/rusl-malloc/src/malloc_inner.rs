//! 内存分配模块 —— Rusl 实现的 libc malloc 相关函数。
//! 根据 spec 文件自动生成的代码骨架。
//!
//! 模块层次结构：
//! - `memalign` — 按对齐边界分配（对外导出 C ABI）
//! - `posix_memalign` — POSIX 对齐分配（对外导出 C ABI）
//! - `free` — 内存释放（对外导出 C ABI）
//! - `realloc` — 重新分配（对外导出 C ABI）
//! - `reallocarray` — 数组安全重分配（对外导出 C ABI）
//! - `libc_calloc` — calloc 系列接口（公共 + 内部）
//! - `calloc_inner` — calloc 内部辅助函数（allzerop, __malloc_replaced, set_errno）
//! - `mallocng` — 新一分配器内部实现（非对外导出）
//! - `lite_malloc` — 精简分配器（非对外导出）
//! - `replaced` — 符号插替检测标志（非对外导出）

#![allow(dead_code, unused_imports)]

// no_std 兼容的 debug_assert: 仅 debug_assertions 启用时检查
// 使用 deliberately_unique 前缀验证此宏确实被展开
macro_rules! debug_assert {
    ($cond:expr $(,)?) => {
        if cfg!(debug_assertions) {
            assert!($cond, "CUSTOM_MACRO_V2: debug_assert failed: {}", stringify!($cond));
        } else {
            let _ = $cond; // 避免 unused 警告
        }
    };
    ($cond:expr, $($arg:tt)+) => {
        if cfg!(debug_assertions) {
            assert!($cond, "CUSTOM_MACRO_V2: debug_assert failed: {}", format_args!($($arg)+));
        } else {
            let _ = $cond;
        }
    };
}

mod libc_calloc;
mod calloc_inner;
pub mod free;
pub(crate) mod lite_malloc;
pub(crate) mod mallocng;
pub mod memalign;
pub mod posix_memalign;
pub mod realloc;
pub mod reallocarray;
pub(crate) mod replaced;

pub(crate) use libc_calloc::__libc_calloc;
pub(crate) use libc_calloc::calloc_impl;
pub(crate) use libc_calloc::mal0_clear;
pub use libc_calloc::calloc;
pub use memalign::memalign;
pub use reallocarray::reallocarray;

// 重导出 mallocng 内部函数 — 供 crate 内部使用
pub use mallocng::malloc::malloc;
pub(crate) use mallocng::free::__libc_free;
pub use mallocng::aligned_alloc::aligned_alloc;
pub use mallocng::malloc_usable_size::malloc_usable_size;
pub(crate) use mallocng::donate::__malloc_donate;
pub(crate) use mallocng::glue::__malloc_atfork;

// 内部辅助函数重导出（供 malloc 子系统的其他模块使用）
pub(crate) use calloc_inner::{allzerop, __malloc_replaced, set_errno, PAGE_SIZE};

// errno 常量：遵循 Linux x86_64 ABI
// Linux/x86_64 上 EINVAL = 22 (参数无效), ENOMEM = 12 (内存不足)
pub(crate) const EINVAL: core::ffi::c_int = 22;
pub(crate) const ENOMEM: core::ffi::c_int = 12;