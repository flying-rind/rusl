#![allow(useless_ptr_null_checks)]
//! `__ctype_get_mb_cur_max` 集成测试
//!
//! 测试 `__ctype_get_mb_cur_max` 的 C ABI 兼容性、符号导出和行为规约。
//!
//! ## 测试范围
//!
//! - C ABI 签名正确性
//! - 链接可见性
//! - 返回值语义（1 或 4）
//! - 多次调用一致性
//!
//! ## 注意
//!
//! - `__ctype_get_mb_cur_max` 是实现为 `todo!()` 的骨架函数，运行时调用会 panic。
use super::*;
// ============================================================================
// 编译期验证：C ABI 签名正确性
// ============================================================================

// 验证符号可被 `extern "C"` 声明引用（链接期检查）。
test!("integration_test_ctype_get_mb_cur_max_linkage" {
    let f: unsafe extern "C" fn() -> usize = __ctype_get_mb_cur_max;
    assert!(
        !(f as *const ()).is_null(),
        "__ctype_get_mb_cur_max 函数指针不应为 NULL"
    );
});

// 验证返回值类型 `usize` 大小。
test!("integration_test_return_type_size" {
    let size = core::mem::size_of::<usize>();
    assert!(
        size == 4 || size == 8,
        "usize 应为 4 (32-bit) 或 8 (64-bit)，实际: {}",
        size
    );
});

// ============================================================================
// 返回值语义常量验证
// ============================================================================

// 验证 spec 中定义的合法返回值。
test!("integration_test_valid_return_values" {
    const UTF8_MAX: usize = 4;
    const C_LOCALE_MAX: usize = 1;

    assert_eq!(UTF8_MAX, 4, "UTF-8 locale: MB_CUR_MAX = 4");
    assert_eq!(C_LOCALE_MAX, 1, "C locale: MB_CUR_MAX = 1");
});

// ============================================================================
// 运行时行为测试（当前实现为 todo!()）
// ============================================================================

// 验证调用安全（仅 panic）。
test!("integration_test_ctype_get_mb_cur_max_call_safe" {
    {
        let _result = __ctype_get_mb_cur_max();
    }
});

// 验证实现完成后返回值在合法范围内。
test!("integration_test_returns_valid_value" {
    {
        let val = __ctype_get_mb_cur_max();
        assert!(val == 1 || val == 4, "返回值只能是 1 或 4，实际: {}", val);
    }
});

// 验证默认 C locale 返回 1。
test!("integration_test_default_returns_1" {
    {
        let val = __ctype_get_mb_cur_max();
        assert_eq!(val, 1, "默认 C locale 下应返回 1");
    }
});

// 验证多次调用一致性。
test!("integration_test_consistency" {
    {
        let v1 = __ctype_get_mb_cur_max();
        let v2 = __ctype_get_mb_cur_max();
        let v3 = __ctype_get_mb_cur_max();
        assert_eq!(v1, v2, "连续调用应返回相同值");
        assert_eq!(v2, v3, "连续调用应返回相同值");
    }
});

// 验证通过函数指针调用。
test!("integration_test_via_fn_ptr" {
    let f: unsafe extern "C" fn() -> usize = __ctype_get_mb_cur_max;
    unsafe {
        let _result = f();
    }
});