//! iswblank 集成测试 —— 从外部调用方角度验证 C ABI 兼容性。
//!
//! 本文件通过 `rusl::ctype::*` 导入符号，验证：
//! - 空格 (0x20) 和水平制表符 (0x09) 识别正确性
//! - 非 blank 字符返回 0（含其他空白字符如换行等）
//! - WEOF 返回 0
//! - `iswblank_l` ABI 兼容性（locale 参数被忽略）
//!
//! ## 注意
//!
//! 当前 `iswblank` 函数体为 `todo!()`，因此所有调用该函数的测试均会 panic。
//! 测试结构、断言和覆盖范围已按 spec 设计完整，待实现完成后即生效。

use super::*;

// ============================================================================
// iswblank 基本功能测试
// ============================================================================

test!("test_iswblank_space" {
    let result = iswblank(b' ' as wint_t);
    assert_ne!(result, 0, "iswblank(' ') 应返回非零值");
});

test!("test_iswblank_tab" {
    let result = iswblank(b'\t' as wint_t);
    assert_ne!(result, 0, "iswblank('\\t') 应返回非零值");
});

// ============================================================================
// iswblank 否定测试 —— 非 blank 字符
// ============================================================================

test!("test_iswblank_newline" {
    // 换行符不是 blank 字符（是 whitespace 但不是 blank）
    let result = iswblank(b'\n' as wint_t);
    assert_eq!(result, 0, "iswblank('\\n') 应返回 0");
});

test!("test_iswblank_carriage_return" {
    let result = iswblank(b'\r' as wint_t);
    assert_eq!(result, 0, "iswblank('\\r') 应返回 0");
});

test!("test_iswblank_vertical_tab" {
    let result = iswblank(0x0Bu32);
    assert_eq!(result, 0, "iswblank('\\v') 应返回 0");
});

test!("test_iswblank_form_feed" {
    let result = iswblank(0x0Cu32);
    assert_eq!(result, 0, "iswblank('\\f') 应返回 0");
});

test!("test_iswblank_letters" {
    for ch in b'A'..=b'Z' {
        let result = iswblank(ch as wint_t);
        assert_eq!(result, 0, "iswblank('{}') 应返回 0", ch as char);
    }
    for ch in b'a'..=b'z' {
        let result = iswblank(ch as wint_t);
        assert_eq!(result, 0, "iswblank('{}') 应返回 0", ch as char);
    }
});

test!("test_iswblank_digits" {
    for ch in b'0'..=b'9' {
        let result = iswblank(ch as wint_t);
        assert_eq!(result, 0, "iswblank('{}') 应返回 0", ch as char);
    }
});

test!("test_iswblank_punctuation" {
    assert_eq!(iswblank(b'!' as wint_t), 0);
    assert_eq!(iswblank(b'.' as wint_t), 0);
    assert_eq!(iswblank(b',' as wint_t), 0);
});

test!("test_iswblank_control_characters" {
    // 除 HT(0x09) 以外的控制字符都应返回 0
    for ch in 0x00u32..=0x08u32 {
        assert_eq!(iswblank(ch), 0, "iswblank(0x{:02X}) 应返回 0", ch);
    }
    // 0x0A-0x1F
    for ch in 0x0Au32..=0x1Fu32 {
        assert_eq!(iswblank(ch), 0, "iswblank(0x{:02X}) 应返回 0", ch);
    }
});

// ============================================================================
// iswblank 特殊值测试
// ============================================================================

test!("test_iswblank_weof" {
    let result = iswblank(WEOF);
    assert_eq!(result, 0, "iswblank(WEOF) 应返回 0");
});

test!("test_iswblank_boundary_before_tab" {
    // 0x08 正好在 HT(0x09) 之前
    assert_eq!(iswblank(0x08u32), 0, "iswblank(0x08) 应返回 0");
    assert_ne!(iswblank(0x09u32), 0, "iswblank(0x09 HT) 应返回非零值");
});

test!("test_iswblank_boundary_after_tab" {
    // 0x0A (LF) 正好在 HT(0x09) 之后
    assert_ne!(iswblank(0x09u32), 0, "iswblank(0x09 HT) 应返回非零值");
    assert_eq!(iswblank(0x0Au32), 0, "iswblank(0x0A LF) 应返回 0");
});

test!("test_iswblank_boundary_around_space" {
    // 0x1F (US) 在空格之前
    assert_eq!(iswblank(0x1Fu32), 0, "iswblank(0x1F) 应返回 0");
    // 0x20 (SP) 是 blank
    assert_ne!(iswblank(0x20u32), 0, "iswblank(' ') 应返回非零值");
    // 0x21 (!) 在空格之后
    assert_eq!(iswblank(0x21u32), 0, "iswblank('!') 应返回 0");
});

// ============================================================================
// iswblank_l 测试（locale 参数被忽略）
// ============================================================================

test!("test_iswblank_l_null_locale" {
    let result = iswblank_l(b' ' as wint_t, core::ptr::null_mut());
    assert_ne!(result, 0, "iswblank_l(' ', NULL) 应返回非零值");
});

test!("test_iswblank_l_tab" {
    let result = iswblank_l(b'\t' as wint_t, core::ptr::null_mut());
    assert_ne!(result, 0, "iswblank_l('\\t', NULL) 应返回非零值");
});

test!("test_iswblank_l_non_blank" {
    let result = iswblank_l(b'A' as wint_t, core::ptr::null_mut());
    assert_eq!(result, 0, "iswblank_l('A', NULL) 应返回 0");
});

test!("test_iswblank_l_weof" {
    let result = iswblank_l(WEOF, core::ptr::null_mut());
    assert_eq!(result, 0, "iswblank_l(WEOF, NULL) 应返回 0");
});

test!("test_iswblank_l_consistency_with_iswblank" {
    // iswblank_l 应与 iswblank 行为完全一致
    for ch in 0x00u32..=0x7Fu32 {
        let result_l = iswblank_l(ch as wint_t, core::ptr::null_mut());
        let result = iswblank(ch as wint_t);
        assert_eq!(
            result_l, result,
            "iswblank_l(0x{:02X}) = {} 应与 iswblank = {} 一致",
            ch, result_l, result
        );
    }
});

// ============================================================================
// 不变量验证
// ============================================================================

test!("test_iswblank_idempotent" {
    for ch in 0x00u32..=0x7Fu32 {
        let r1 = iswblank(ch as wint_t);
        let r2 = iswblank(ch as wint_t);
        assert_eq!(r1, r2, "iswblank(0x{:02X}) 多次调用应返回相同结果", ch);
    }
});

test!("test_iswblank_returns_only_zero_or_one" {
    // musl 中 iswblank 仅返回 0 或 1
    for ch in 0x00u32..=0x7Fu32 {
        let result = iswblank(ch as wint_t);
        assert!(result == 0 || result == 1, "iswblank(0x{:02X}) 返回值 {} 应为 0 或 1", ch, result);
    }
});

// ============================================================================
// iswblank vs isspace 区别验证
// ============================================================================

test!("test_iswblank_only_space_and_tab" {
    // iswblank 仅识别空格和水平制表符，与 isspace 不同
    // iswblank 不应识别换行、垂直制表、换页、回车等 isspace 识别的字符
    let isspace_but_not_blank: [wint_t; 4] = [
        b'\n' as wint_t, // LF
        0x0B,             // VT
        0x0C,             // FF
        b'\r' as wint_t, // CR
    ];

    for &ch in &isspace_but_not_blank {
        let result = iswblank(ch);
        assert_eq!(result, 0, "iswblank(0x{:02X}) 应返回 0（这是 isspace 字符但不是 blank）", ch);
    }
});