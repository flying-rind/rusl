//! iswalnum 集成测试 —— 从外部调用方角度验证 C ABI 兼容性。
//!
//! 本文件通过 `rusl::ctype::*` 导入符号，验证：
//! - ASCII 十进制数字 '0'-'9' 识别正确性
//! - ASCII 字母 'A'-'Z', 'a'-'z' 识别正确性
//! - 非字母非数字字符返回 0
//! - WEOF 返回 0
//! - `iswalnum_l` ABI 兼容性（locale 参数被忽略）
//!
//! ## 注意
//!
//! 当前 `iswalnum` 函数体为 `todo!()`，因此所有调用该函数的测试均会 panic。
//! 测试结构、断言和覆盖范围已按 spec 设计完整，待实现完成后即生效。


use super::*;

// ============================================================================
// iswalnum 基本功能测试 —— ASCII 数字
// ============================================================================

test!("test_iswalnum_ascii_digits" {
    for ch in b'0'..=b'9' {
        let result = iswalnum(ch as wint_t);
        assert_ne!(result, 0, "iswalnum('{}') 应返回非零值", ch as char);
    }
});

test!("test_iswalnum_digit_zero" {
    let result = iswalnum(b'0' as wint_t);
    assert_ne!(result, 0, "iswalnum('0') 应返回非零值");
});

test!("test_iswalnum_digit_nine" {
    let result = iswalnum(b'9' as wint_t);
    assert_ne!(result, 0, "iswalnum('9') 应返回非零值");
});

// ============================================================================
// iswalnum 基本功能测试 —— ASCII 字母
// ============================================================================

test!("test_iswalnum_ascii_uppercase" {
    for ch in b'A'..=b'Z' {
        let result = iswalnum(ch as wint_t);
        assert_ne!(result, 0, "iswalnum('{}') 应返回非零值", ch as char);
    }
});

test!("test_iswalnum_ascii_lowercase" {
    for ch in b'a'..=b'z' {
        let result = iswalnum(ch as wint_t);
        assert_ne!(result, 0, "iswalnum('{}') 应返回非零值", ch as char);
    }
});

// ============================================================================
// iswalnum 否定测试 —— 非字母非数字
// ============================================================================

test!("test_iswalnum_non_alnum_punctuation" {
    assert_eq!(iswalnum(b'!' as wint_t), 0);
    assert_eq!(iswalnum(b'@' as wint_t), 0);
    assert_eq!(iswalnum(b'[' as wint_t), 0);
    assert_eq!(iswalnum(b'`' as wint_t), 0);
    assert_eq!(iswalnum(b'{' as wint_t), 0);
    assert_eq!(iswalnum(b'~' as wint_t), 0);
});

test!("test_iswalnum_non_alnum_space" {
    assert_eq!(iswalnum(b' ' as wint_t), 0, "iswalnum(' ') 应返回 0");
});

test!("test_iswalnum_non_alnum_control" {
    assert_eq!(iswalnum(b'\t' as wint_t), 0);
    assert_eq!(iswalnum(b'\n' as wint_t), 0);
    assert_eq!(iswalnum(0x00u32), 0, "iswalnum(NUL) 应返回 0");
});

// ============================================================================
// iswalnum 特殊值测试
// ============================================================================

test!("test_iswalnum_weof" {
    let result = iswalnum(WEOF);
    assert_eq!(result, 0, "iswalnum(WEOF) 应返回 0");
});

test!("test_iswalnum_boundary_before_digit_zero" {
    // '/' = 0x2F, 正好在 '0'(0x30) 之前
    assert_eq!(iswalnum(b'/' as wint_t), 0, "iswalnum('/') 应返回 0");
    assert_ne!(iswalnum(b'0' as wint_t), 0, "iswalnum('0') 应返回非零值");
});

test!("test_iswalnum_boundary_after_digit_nine" {
    // ':' = 0x3A, 正好在 '9'(0x39) 之后
    assert_ne!(iswalnum(b'9' as wint_t), 0, "iswalnum('9') 应返回非零值");
    assert_eq!(iswalnum(b':' as wint_t), 0, "iswalnum(':') 应返回 0");
});

// ============================================================================
// iswalnum_l 测试（locale 参数被忽略）
// ============================================================================

test!("test_iswalnum_l_null_locale" {
    let result = iswalnum_l(b'A' as wint_t, core::ptr::null_mut());
    assert_ne!(result, 0, "iswalnum_l('A', NULL) 应返回非零值");
});

test!("test_iswalnum_l_digit" {
    let result = iswalnum_l(b'5' as wint_t, core::ptr::null_mut());
    assert_ne!(result, 0, "iswalnum_l('5', NULL) 应返回非零值");
});

test!("test_iswalnum_l_non_alnum" {
    let result = iswalnum_l(b'!' as wint_t, core::ptr::null_mut());
    assert_eq!(result, 0, "iswalnum_l('!', NULL) 应返回 0");
});

test!("test_iswalnum_l_weof" {
    let result = iswalnum_l(WEOF, core::ptr::null_mut());
    assert_eq!(result, 0, "iswalnum_l(WEOF, NULL) 应返回 0");
});

test!("test_iswalnum_l_consistency_with_iswalnum" {
    // iswalnum_l 应与 iswalnum 行为完全一致（验证整个 ASCII 范围）
    for ch in 0x00u32..=0x7Fu32 {
        let result_l = iswalnum_l(ch as wint_t, core::ptr::null_mut());
        let result = iswalnum(ch as wint_t);
        assert_eq!(
            result_l, result,
            "iswalnum_l(0x{:02X}) = {} 应与 iswalnum = {} 一致",
            ch, result_l, result
        );
    }
});

// ============================================================================
// 不变量验证
// ============================================================================

test!("test_iswalnum_idempotent" {
    for ch in 0x00u32..=0x7Fu32 {
        let r1 = iswalnum(ch as wint_t);
        let r2 = iswalnum(ch as wint_t);
        assert_eq!(r1, r2, "iswalnum(0x{:02X}) 多次调用应返回相同结果", ch);
    }
});

test!("test_iswalnum_returns_only_zero_or_one" {
    // musl 中 iswalnum 仅返回 0 或 1
    for ch in 0x00u32..=0x7Fu32 {
        let result = iswalnum(ch as wint_t);
        assert!(result == 0 || result == 1, "iswalnum(0x{:02X}) 返回值 {} 应为 0 或 1", ch, result);
    }
});

// ============================================================================
// 数字优先路径验证
// ============================================================================

test!("test_iswalnum_digit_range_continuity" {
    // 验证 '0'-'9' 连续被识别为 alnum
    for ch in b'0'..=b'9' {
        assert_ne!(iswalnum(ch as wint_t), 0);
    }
});

test!("test_iswalnum_letter_range_continuity" {
    // 验证 'A'-'Z' 和 'a'-'z' 连续被识别为 alnum
    for ch in b'A'..=b'Z' {
        assert_ne!(iswalnum(ch as wint_t), 0);
    }
    for ch in b'a'..=b'z' {
        assert_ne!(iswalnum(ch as wint_t), 0);
    }
});