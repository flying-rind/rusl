//! iswalpha 集成测试 —— 从外部调用方角度验证 C ABI 兼容性。
//!
//! 本文件通过 `rusl::ctype::*` 导入符号，验证：
//! - ASCII 字母 'A'-'Z', 'a'-'z' 识别正确性
//! - Unicode 字母码点识别（代表性码点采样）
//! - CJK Extension B 范围（U+20000-U+2FFFD）正确返回 1
//! - 非字母字符返回 0
//! - WEOF 返回 0
//! - `iswalpha_l` ABI 兼容性（locale 参数被忽略）
//!
//! ## 注意
//!
//! 当前 `iswalpha` 函数体为 `todo!()`，因此所有调用该函数的测试均会 panic。
//! 测试结构、断言和覆盖范围已按 spec 设计完整，待实现完成后即生效。

use super::*;

// ============================================================================
// iswalpha 基本功能测试 —— ASCII 字母
// ============================================================================

test!("test_iswalpha_ascii_uppercase" {
    for ch in b'A'..=b'Z' {
        let result = iswalpha(ch as wint_t);
        assert_ne!(result, 0, "iswalpha('{}') 应返回非零值", ch as char);
    }
});

test!("test_iswalpha_ascii_lowercase" {
    for ch in b'a'..=b'z' {
        let result = iswalpha(ch as wint_t);
        assert_ne!(result, 0, "iswalpha('{}') 应返回非零值", ch as char);
    }
});

// ============================================================================
// iswalpha 否定测试 —— 非字母字符
// ============================================================================

test!("test_iswalpha_non_alpha_digits" {
    for ch in b'0'..=b'9' {
        let result = iswalpha(ch as wint_t);
        assert_eq!(result, 0, "iswalpha('{}') 应返回 0", ch as char);
    }
});

test!("test_iswalpha_non_alpha_punctuation" {
    assert_eq!(iswalpha(b'!' as wint_t), 0);
    assert_eq!(iswalpha(b'@' as wint_t), 0);
    assert_eq!(iswalpha(b'[' as wint_t), 0);
    assert_eq!(iswalpha(b'`' as wint_t), 0);
    assert_eq!(iswalpha(b'{' as wint_t), 0);
});

test!("test_iswalpha_non_alpha_space_and_control" {
    assert_eq!(iswalpha(b' ' as wint_t), 0);
    assert_eq!(iswalpha(b'\t' as wint_t), 0);
    assert_eq!(iswalpha(b'\n' as wint_t), 0);
    assert_eq!(iswalpha(0x00u32), 0);
});

// ============================================================================
// iswalpha 边界测试（ASCII 区间边界）
// ============================================================================

test!("test_iswalpha_boundary_before_uppercase_a" {
    // '@'(0x40) 正好在 'A'(0x41) 之前
    assert_eq!(iswalpha(0x40u32), 0, "iswalpha('@') 应返回 0");
    assert_ne!(iswalpha(b'A' as wint_t), 0, "iswalpha('A') 应返回非零值");
});

test!("test_iswalpha_boundary_after_uppercase_z" {
    // '['(0x5B) 正好在 'Z'(0x5A) 之后
    assert_ne!(iswalpha(b'Z' as wint_t), 0, "iswalpha('Z') 应返回非零值");
    assert_eq!(iswalpha(0x5Bu32), 0, "iswalpha('[') 应返回 0");
});

test!("test_iswalpha_boundary_before_lowercase_a" {
    // '`'(0x60) 正好在 'a'(0x61) 之前
    assert_eq!(iswalpha(0x60u32), 0, "iswalpha('`') 应返回 0");
    assert_ne!(iswalpha(b'a' as wint_t), 0, "iswalpha('a') 应返回非零值");
});

test!("test_iswalpha_boundary_after_lowercase_z" {
    // '{'(0x7B) 正好在 'z'(0x7A) 之后
    assert_ne!(iswalpha(b'z' as wint_t), 0, "iswalpha('z') 应返回非零值");
    assert_eq!(iswalpha(0x7Bu32), 0, "iswalpha('{{') 应返回 0");
});

// ============================================================================
// iswalpha Unicode 字母码点测试（代表性采样）
// ============================================================================

test!("test_iswalpha_unicode_latin_supplement" {
    // Latin-1 Supplement 区段字母（代表性）
    // U+00C0 (LATIN CAPITAL LETTER A WITH GRAVE)
    assert_ne!(iswalpha(0x00C0u32), 0, "iswalpha(U+00C0) 应返回非零值");
    // U+00E0 (LATIN SMALL LETTER A WITH GRAVE)
    assert_ne!(iswalpha(0x00E0u32), 0, "iswalpha(U+00E0) 应返回非零值");
});

test!("test_iswalpha_unicode_greek" {
    // Greek and Coptic 区段
    // U+0391 (GREEK CAPITAL LETTER ALPHA)
    assert_ne!(iswalpha(0x0391u32), 0, "iswalpha(U+0391) 应返回非零值");
    // U+03B1 (GREEK SMALL LETTER ALPHA)
    assert_ne!(iswalpha(0x03B1u32), 0, "iswalpha(U+03B1) 应返回非零值");
});

test!("test_iswalpha_unicode_cyrillic" {
    // Cyrillic 区段
    // U+0410 (CYRILLIC CAPITAL LETTER A)
    assert_ne!(iswalpha(0x0410u32), 0, "iswalpha(U+0410) 应返回非零值");
    // U+0430 (CYRILLIC SMALL LETTER A)
    assert_ne!(iswalpha(0x0430u32), 0, "iswalpha(U+0430) 应返回非零值");
});

test!("test_iswalpha_unicode_cjk" {
    // CJK Unified Ideographs 区段
    // U+4E00 (CJK UNIFIED IDEOGRAPH-4E00, 一)
    assert_ne!(iswalpha(0x4E00u32), 0, "iswalpha(U+4E00) 应返回非零值");
    // U+9FA5 (CJK UNIFIED IDEOGRAPH-9FA5)
    assert_ne!(iswalpha(0x9FA5u32), 0, "iswalpha(U+9FA5) 应返回非零值");
});

// ============================================================================
// iswalpha Phase 2 测试 —— CJK Extension B（U+20000-U+2FFFD）
// ============================================================================

test!("test_iswalpha_cjk_ext_b_start" {
    // U+20000 是 CJK Extension B 的起点，应返回 1
    let result = iswalpha(0x20000u32);
    assert_ne!(result, 0, "iswalpha(U+20000) CJK Ext-B 起点应返回非零值");
});

test!("test_iswalpha_cjk_ext_b_middle" {
    // U+24000 在 CJK Extension B 中间
    let result = iswalpha(0x24000u32);
    assert_ne!(result, 0, "iswalpha(U+24000) CJK Ext-B 中部应返回非零值");
});

test!("test_iswalpha_cjk_ext_b_end" {
    // U+2FFFD 是 CJK Extension B 的最后一个码点（spec 说 < 0x2FFFE）
    let result = iswalpha(0x2FFFDu32);
    assert_ne!(result, 0, "iswalpha(U+2FFFD) CJK Ext-B 终点应返回非零值");
});

// ============================================================================
// iswalpha Phase 3 测试 —— 越界
// ============================================================================

test!("test_iswalpha_phase3_boundary" {
    // U+2FFFE 正好在 CJK Extension B 范围之后，应返回 0
    let result = iswalpha(0x2FFFEu32);
    assert_eq!(result, 0, "iswalpha(U+2FFFE) 越界应返回 0");
});

test!("test_iswalpha_phase3_large_value" {
    // U+30000 远超范围，应返回 0
    let result = iswalpha(0x30000u32);
    assert_eq!(result, 0, "iswalpha(U+30000) 应返回 0");
});

// ============================================================================
// iswalpha 特殊值测试
// ============================================================================

test!("test_iswalpha_weof" {
    let result = iswalpha(WEOF);
    assert_eq!(result, 0, "iswalpha(WEOF) 应返回 0");
});

test!("test_iswalpha_bmp_boundary" {
    // U+1FFFF 是 Phase 1 位图查找的最大值（< 0x20000）
    // 此处仅验证调用不崩溃，实际返回值依赖位图数据
    let _result = iswalpha(0x1FFFFu32);
    // 不检查具体返回值，因为依赖 TABLE 数据（当前为占位符）
});

// ============================================================================
// iswalpha_l 测试（locale 参数被忽略）
// ============================================================================

test!("test_iswalpha_l_null_locale" {
    let result = iswalpha_l(b'A' as wint_t, core::ptr::null_mut());
    assert_ne!(result, 0, "iswalpha_l('A', NULL) 应返回非零值");
});

test!("test_iswalpha_l_non_alpha" {
    let result = iswalpha_l(b'1' as wint_t, core::ptr::null_mut());
    assert_eq!(result, 0, "iswalpha_l('1', NULL) 应返回 0");
});

test!("test_iswalpha_l_weof" {
    let result = iswalpha_l(WEOF, core::ptr::null_mut());
    assert_eq!(result, 0, "iswalpha_l(WEOF, NULL) 应返回 0");
});

test!("test_iswalpha_l_consistency_with_iswalpha" {
    // iswalpha_l 应与 iswalpha 行为完全一致（验证 ASCII 范围）
    for ch in 0x00u32..=0x7Fu32 {
        let result_l = iswalpha_l(ch as wint_t, core::ptr::null_mut());
        let result = iswalpha(ch as wint_t);
        assert_eq!(
            result_l, result,
            "iswalpha_l(0x{:02X}) = {} 应与 iswalpha = {} 一致",
            ch, result_l, result
        );
    }
});

// ============================================================================
// 不变量验证
// ============================================================================

test!("test_iswalpha_idempotent" {
    for ch in 0x00u32..=0x7Fu32 {
        let r1 = iswalpha(ch as wint_t);
        let r2 = iswalpha(ch as wint_t);
        assert_eq!(r1, r2, "iswalpha(0x{:02X}) 多次调用应返回相同结果", ch);
    }
});

test!("test_iswalpha_returns_only_zero_or_one" {
    // musl 中 iswalpha 仅返回 0 或 1
    for ch in 0x00u32..=0x7Fu32 {
        let result = iswalpha(ch as wint_t);
        assert!(result == 0 || result == 1, "iswalpha(0x{:02X}) 返回值 {} 应为 0 或 1", ch, result);
    }
});