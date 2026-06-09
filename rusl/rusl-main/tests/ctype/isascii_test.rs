#![allow(useless_ptr_null_checks)]
//! `isascii` 集成测试
//!
//! 测试 `isascii` 的 C ABI 兼容性和行为规约。
//!
//! ## 注意
//!
//! - `isascii` 是实现为 `todo!()` 的骨架函数，运行时调用会 panic。

use core::ffi::c_int;
use super::*;

// ============================================================================
// 编译期验证
// ============================================================================

test!("integration_test_isascii_linkage" {
    let f: unsafe extern "C" fn(c_int) -> c_int = isascii;
    assert!(!(f as *const ()).is_null());
});

test!("integration_test_c_int_size" {
    assert_eq!(core::mem::size_of::<c_int>(), 4);
});

// ============================================================================
// ASCII 范围常量验证
// ============================================================================

test!("integration_test_ascii_range_definition" {
    assert_eq!(0x7f, 127);
    // 128 个 ASCII 字符（0-127）
    let mut count = 0u32;
    for _c in 0i32..128 {
        count = count.wrapping_add(1);
    }
    assert_eq!(count, 128);
});

test!("integration_test_bitmask" {
    let not_7f: i32 = !0x7f;
    for c in 0..128i32 {
        assert_eq!(c & not_7f, 0, "ASCII {} & !0x7f == 0", c);
    }
    for c in 128..256i32 {
        assert_ne!(c & not_7f, 0, "非 ASCII {} & !0x7f != 0", c);
    }
});

// ============================================================================
// 运行时行为测试（当前为 todo!()）
// ============================================================================

test!("integration_test_isascii_basic" {
    { let _ = isascii(b'A' as c_int); }
});

test!("integration_test_ascii_range_true" {
    {
        for c in 0..128i32 {
            assert_ne!(isascii(c), 0, "isascii({}) != 0", c);
        }
    }
});

test!("integration_test_extended_ascii_false" {
    {
        for c in 128..256i32 {
            assert_eq!(isascii(c), 0, "isascii({}) == 0", c);
        }
    }
});

test!("integration_test_boundary_0" {
    { assert_ne!(isascii(0), 0); }
});

test!("integration_test_boundary_127" {
    { assert_ne!(isascii(127), 0); }
});

test!("integration_test_boundary_128" {
    { assert_eq!(isascii(128), 0); }
});

test!("integration_test_boundary_255" {
    { assert_eq!(isascii(255), 0); }
});

test!("integration_test_negative_values" {
    {
        for c in [-1, -2, -10, -128, -255].iter() {
            assert_eq!(isascii(*c), 0, "isascii({}) == 0", c);
        }
    }
});

test!("integration_test_eof" {
    { assert_eq!(isascii(-1), 0, "isascii(EOF) == 0"); }
});

test!("integration_test_large_values" {
    {
        assert_eq!(isascii(256), 0);
        assert_eq!(isascii(1024), 0);
        assert_eq!(isascii(i32::MAX), 0);
    }
});

test!("integration_test_via_fn_ptr" {
    let f: unsafe extern "C" fn(c_int) -> c_int = isascii;
    unsafe { let _ = f(b'Z' as c_int); }
});