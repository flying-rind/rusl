//! isblank 集成测试 —— 从外部调用方角度验证 C ABI 兼容性。
//!
//! 本文件通过 `rusl::ctype::*` 导入符号，验证：
//! - 空格 (0x20) 和水平制表符 (0x09) 识别正确性
//! - 非空白字符返回 0
//! - EOF (-1) 返回 0
//! - `isblank_l` ABI 兼容性（locale 参数被忽略）
//!
//! ## 注意
//!
//! 当前 `isblank` 函数体为 `todo!()`，因此所有调用该函数的测试均会 panic。
//! 测试结构、断言和覆盖范围已按 spec 设计完整，待实现完成后即生效。

use core::ffi::c_int;
use super::*;

// ============================================================================
// 常量
// ============================================================================

/// C 标准 EOF 值。
const EOF: c_int = -1;

/// 空格字符的整数值。
const CHAR_SPACE: c_int = 0x20;

/// 水平制表符的整数值。
const CHAR_TAB: c_int = 0x09;

// ============================================================================
// isblank 基本功能测试
// ============================================================================

test!("test_isblank_space" {
    let result = isblank(CHAR_SPACE);
    assert_ne!(result, 0, "isblank(' ') 应返回非零值");
});

test!("test_isblank_horizontal_tab" {
    let result = isblank(CHAR_TAB);
    assert_ne!(result, 0, "isblank('\\t') 应返回非零值");
});

test!("test_isblank_other_whitespace" {
    // 换行、垂直制表、换页、回车不是 blank
    assert_eq!(isblank(b'\n' as c_int), 0, "isblank('\\n') 应为 0");
    assert_eq!(isblank(0x0B), 0, "isblank(0x0B) 应为 0");
    assert_eq!(isblank(0x0C), 0, "isblank(0x0C) 应为 0");
    assert_eq!(isblank(b'\r' as c_int), 0, "isblank('\\r') 应为 0");
});

test!("test_isblank_non_blank_characters" {
    // 字母、数字、标点都不是 blank
    assert_eq!(isblank(b'a' as c_int), 0);
    assert_eq!(isblank(b'Z' as c_int), 0);
    assert_eq!(isblank(b'0' as c_int), 0);
    assert_eq!(isblank(b'!' as c_int), 0);
});

// ============================================================================
// isblank 边界测试
// ============================================================================

test!("test_isblank_boundary_before_tab" {
    // 0x08 在 \t (0x09) 之前
    let result = isblank(0x08);
    assert_eq!(result, 0, "isblank(0x08) 应返回 0（位于 tab 之前）");
});

test!("test_isblank_boundary_after_tab" {
    // 0x0A 在 \t (0x09) 之后
    let result = isblank(0x0A);
    assert_eq!(result, 0, "isblank(0x0A) 应返回 0（位于 tab 之后）");
});

test!("test_isblank_boundary_before_space" {
    // 0x1F 在空格 (0x20) 之前
    let result = isblank(0x1F);
    assert_eq!(result, 0, "isblank(0x1F) 应返回 0（位于空格之前）");
});

test!("test_isblank_boundary_first_blank" {
    // 0x09 (\t) 是第一个空白字符
    let result = isblank(CHAR_TAB);
    assert_ne!(result, 0, "isblank(0x09) 应返回非零值");
});

test!("test_isblank_boundary_second_blank" {
    // 0x20 (' ') 是第二个空白字符
    let result = isblank(CHAR_SPACE);
    assert_ne!(result, 0, "isblank(0x20) 应返回非零值");
});

test!("test_isblank_boundary_after_space" {
    // 0x21 在空格 (0x20) 之后
    let result = isblank(0x21);
    assert_eq!(result, 0, "isblank(0x21) 应返回 0（位于空格之后）");
});

test!("test_isblank_eof" {
    let result = isblank(EOF);
    assert_eq!(result, 0, "isblank(EOF) 应返回 0");
});

test!("test_isblank_nul" {
    let result = isblank(0x00);
    assert_eq!(result, 0, "isblank(NUL) 应返回 0");
});

test!("test_isblank_del" {
    let result = isblank(0x7F);
    assert_eq!(result, 0, "isblank(DEL) 应返回 0");
});

test!("test_isblank_extended_ascii" {
    // 扩展 ASCII (> 0x7F) 不应是空白字符
    for ch in [0x80u8, 0xC0, 0xFF].iter() {
        assert_eq!(isblank(*ch as c_int), 0, "isblank(0x{:02X}) 应为 0", ch);
    }
});

// ============================================================================
// isblank_l 测试（locale 参数被忽略）
// ============================================================================

test!("test_isblank_l_null_locale" {
    let result = isblank_l(CHAR_SPACE, core::ptr::null_mut());
    assert_ne!(result, 0, "isblank_l(' ', NULL) 应返回非零值");
});

test!("test_isblank_l_tab" {
    let result = isblank_l(CHAR_TAB, core::ptr::null_mut());
    assert_ne!(result, 0, "isblank_l('\\t', NULL) 应返回非零值");
});

test!("test_isblank_l_non_blank" {
    let result = isblank_l(b'x' as c_int, core::ptr::null_mut());
    assert_eq!(result, 0, "isblank_l('x', NULL) 应返回 0");
});

test!("test_isblank_l_eof" {
    let result = isblank_l(EOF, core::ptr::null_mut());
    assert_eq!(result, 0, "isblank_l(EOF, NULL) 应返回 0");
});

test!("test_isblank_l_consistency_with_isblank" {
    // isblank_l 应与 isblank 行为完全一致
    for ch in 0x00..=0xFF {
        let result_l = isblank_l(ch, core::ptr::null_mut());
        let result = isblank(ch);
        assert_eq!(
            result_l, result,
            "isblank_l(0x{:02X}) = {} 应与 isblank = {} 一致",
            ch, result_l, result
        );
    }
});

// ============================================================================
// 不变量验证
// ============================================================================

test!("test_isblank_idempotent" {
    // isblank 是纯函数，多次调用应返回相同结果
    for ch in 0x00..=0xFF {
        let r1 = isblank(ch);
        let r2 = isblank(ch);
        assert_eq!(r1, r2, "isblank(0x{:02X}) 多次调用应返回相同结果", ch);
    }
});

test!("test_isblank_returns_only_zero_or_one" {
    // musl 中 isblank 仅返回 0 或 1
    for ch in 0x00..=0xFF {
        let result = isblank(ch);
        assert!(result == 0 || result == 1, "isblank(0x{:02X}) 返回值 {} 应为 0 或 1", ch, result);
    }
});

test!("test_isblank_only_two_blanks" {
    // 验证恰好有两个空白字符：空格和水平制表符
    let count: i32 = (0x00..=0xFF)
        .map(|ch| if isblank(ch) != 0 { 1 } else { 0 })
        .sum();
    assert_eq!(count, 2, "恰好应有 2 个空白字符，实际得到 {}", count);
});