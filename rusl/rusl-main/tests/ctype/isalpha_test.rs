#![allow(useless_ptr_null_checks)]
//! `isalpha` 集成测试
//!
//! 测试 `isalpha`、`isalpha_l` 的 C ABI 兼容性和行为规约。
//!
//! ## 测试范围
//!
//! - C ABI 签名正确性
//! - 字母/非字母字符分类验证
//! - 边界条件测试
//! - locale 参数一致性验证
//!
//! ## 注意
//!
//! - isalpha 族函数均使用 `todo!()`，运行时调用会 panic。

use core::ffi::c_int;
use super::*;

// ============================================================================
// 编译期验证
// ============================================================================

test!("integration_test_isalpha_linkage" {
    let f: unsafe extern "C" fn(c_int) -> c_int = isalpha;
    assert!(!(f as *const ()).is_null());
});

test!("integration_test_isalpha_l_public_linkage" {
    let f: unsafe extern "C" fn(c_int, locale_t) -> c_int = isalpha_l;
    assert!(!(f as *const ()).is_null());
});

test!("integration_test_locale_t_size" {
    let size = core::mem::size_of::<locale_t>();
    assert!(size == 4 || size == 8, "locale_t 指针大小: {}", size);
});

test!("integration_test_c_int_size" {
    assert_eq!(core::mem::size_of::<c_int>(), 4, "c_int 应占 4 字节");
});

// ============================================================================
// 字母范围常量验证
// ============================================================================

test!("integration_test_alpha_ascii_ranges" {
    assert_eq!(b'A', 65);
    assert_eq!(b'Z', 90);
    assert_eq!(b'a', 97);
    assert_eq!(b'z', 122);
    assert_eq!(b'Z' - b'A' + 1, 26);
    assert_eq!(b'z' - b'a' + 1, 26);
});

test!("integration_test_or_32_converts_to_lowercase" {
    for upper in b'A'..=b'Z' {
        assert_eq!(upper | 32, upper + 32);
        assert!(upper | 32 >= b'a' && upper | 32 <= b'z');
    }
});

// ============================================================================
// 运行时行为测试（当前为 todo!()）
// ============================================================================

// isalpha 基本调用
test!("integration_test_isalpha_basic" {
    {
        let _r = isalpha(b'a' as c_int);
    }
});

// isalpha 小写字母 a-z
test!("integration_test_isalpha_lowercase_range" {
    {
        for c in b'a'..=b'z' {
            assert_ne!(isalpha(c as c_int), 0, "isalpha('{}') != 0", c as char);
        }
    }
});

// isalpha 大写字母 A-Z
test!("integration_test_isalpha_uppercase_range" {
    {
        for c in b'A'..=b'Z' {
            assert_ne!(isalpha(c as c_int), 0, "isalpha('{}') != 0", c as char);
        }
    }
});

// isalpha 数字返回 0
test!("integration_test_isalpha_digits_zero" {
    {
        for c in b'0'..=b'9' {
            assert_eq!(isalpha(c as c_int), 0, "isalpha('{}') == 0", c as char);
        }
    }
});

// isalpha EOF 返回 0
test!("integration_test_isalpha_eof" {
    {
        assert_eq!(isalpha(-1), 0, "isalpha(EOF) == 0");
    }
});

// isalpha 边界测试
test!("integration_test_isalpha_boundaries" {
    {
        assert_eq!(isalpha(b'@' as c_int), 0, "'@' 在 'A' 前");
        assert_eq!(isalpha(b'[' as c_int), 0, "'[' 在 'Z' 后");
        assert_eq!(isalpha(b'`' as c_int), 0, "'`' 在 'a' 前");
        assert_eq!(isalpha(b'{' as c_int), 0, "'{{' 在 'z' 后");
    }
});

// 通过函数指针调用
test!("integration_test_via_fn_ptr" {
    let f: unsafe extern "C" fn(c_int) -> c_int = isalpha;
    unsafe { let _ = f(b'a' as c_int); }
});