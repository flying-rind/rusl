//! dynlink 模块 — 动态链接器替换标志。
//! [Visibility]: Internal (pub(crate))
//!
//! 对应 musl 的 `src/malloc/mallocng/glue.h` 中引用的动态链接符号。
//! 提供 `malloc`/`aligned_alloc` 是否被外部动态库替换的标志位。
//!
//! # rusl no_std 环境说明
//!
//! rusl 为 `#![no_std]` 静态链接库，不存在动态链接器替换场景。
//! 这些标志始终为 `false`，保留仅为与 C 代码结构语义一致。

use core::sync::atomic::AtomicBool;

/// 标记 `malloc` 是否被外部动态库替换。
///
/// 在支持动态链接的 musl 中，当用户程序或预加载库提供了自定义 `malloc` 时，
/// 此标志被设置为 `true`。
///
/// rusl `no_std` 环境下始终为 `false`。
#[allow(non_upper_case_globals)]
pub static __malloc_replaced: AtomicBool = AtomicBool::new(false);

/// 标记 `aligned_alloc` 是否被外部动态库替换。
///
/// 在支持动态链接的 musl 中，当用户程序或预加载库提供了自定义 `aligned_alloc` 时，
/// 此标志被设置为 `true`。
///
/// rusl `no_std` 环境下始终为 `false`。
#[allow(non_upper_case_globals)]
pub static __aligned_alloc_replaced: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;
    use core::sync::atomic::Ordering;

    test!("test_malloc_replaced_default_false" {
        // 验证默认值为 false (rusl no_std 环境)
        assert!(!__malloc_replaced.load(Ordering::Relaxed));
    });

    test!("test_aligned_alloc_replaced_default_false" {
        // 验证默认值为 false (rusl no_std 环境)
        assert!(!__aligned_alloc_replaced.load(Ordering::Relaxed));
    });

    test!("test_both_replaced_false_implies_aligned_alloc_enabled" {
        // 验证: 当两个标志均为 false 时，aligned_alloc 可用
        assert!(!__malloc_replaced.load(Ordering::Relaxed));
        assert!(!__aligned_alloc_replaced.load(Ordering::Relaxed));
        // is_aligned_alloc_disabled() 应返回 false
        // (实际验证需等 glue 模块实现后补充)
    });
}