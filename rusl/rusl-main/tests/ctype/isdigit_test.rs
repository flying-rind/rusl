//! isdigit 集成测试 —— 从外部调用方角度验证 C ABI 兼容性。
//!
//! 本文件通过 `rusl::ctype::*` 导入符号，验证：
//! - 全部 10 个十进制数字 ('0'-'9') 识别正确性
//! - 非数字字符返回 0
//! - EOF (-1) 返回 0
//! - `isdigit_l` ABI 兼容性（locale 参数被忽略）
//!
//! ## 注意
//!
//! 当前 `isdigit` 函数体为 `todo!()`，因此所有调用该函数的测试均会 panic。
//! 测试结构、断言和覆盖范围已按 spec 设计完整，待实现完成后即生效。

use core::ffi::c_int;
use super::*;

// ============================================================================
// 常量
// ============================================================================

/// C 标准 EOF 值。
const EOF: c_int = -1;

/// 第一个数字字符 '0' 的整数值。
const CHAR_0: c_int = b'0' as c_int;

/// 最后一个数字字符 '9' 的整数值。
const CHAR_9: c_int = b'9' as c_int;

// ============================================================================
// isdigit 基本功能测试
// ============================================================================

test!("test_isdigit_all_digits" {
    for ch in CHAR_0..=CHAR_9 {
        let result = isdigit(ch);
        assert_ne!(result, 0, "isdigit('{}') 应返回非零值", ch as u8 as char);
    }
});

test!("test_isdigit_non_digit_lowercase" {
    for ch in b'a'..=b'z' {
        let result = isdigit(ch as c_int);
        assert_eq!(result, 0, "isdigit('{}') 应为 0", ch as char);
    }
});

test!("test_isdigit_non_digit_uppercase" {
    for ch in b'A'..=b'Z' {
        let result = isdigit(ch as c_int);
        assert_eq!(result, 0, "isdigit('{}') 应为 0", ch as char);
    }
});

test!("test_isdigit_whitespace" {
    assert_eq!(isdigit(b' ' as c_int), 0, "isdigit(' ') 应为 0");
    assert_eq!(isdigit(b'\t' as c_int), 0, "isdigit('\\t') 应为 0");
    assert_eq!(isdigit(b'\n' as c_int), 0, "isdigit('\\n') 应为 0");
});

test!("test_isdigit_punctuation" {
    assert_eq!(isdigit(b'!' as c_int), 0);
    assert_eq!(isdigit(b'@' as c_int), 0);
    assert_eq!(isdigit(b'#' as c_int), 0);
});

test!("test_isdigit_hex_letters_not_digit" {
    // 十六进制字母 A-F、a-f 不是十进制数字
    for &ch in &[b'A' as c_int, b'F' as c_int, b'a' as c_int, b'f' as c_int] {
        assert_eq!(isdigit(ch), 0, "isdigit('{}') 应为 0", ch as u8 as char);
    }
});

// ============================================================================
// isdigit 边界测试
// ============================================================================

test!("test_isdigit_boundary_before_zero" {
    // '/' (0x2F) 正好在 '0' (0x30) 之前
    let result = isdigit(b'/' as c_int);
    assert_eq!(result, 0, "isdigit('/') 应返回 0");
});

test!("test_isdigit_boundary_first_digit" {
    let result = isdigit(CHAR_0);
    assert_ne!(result, 0, "isdigit('0') 应返回非零值");
});

test!("test_isdigit_boundary_last_digit" {
    let result = isdigit(CHAR_9);
    assert_ne!(result, 0, "isdigit('9') 应返回非零值");
});

test!("test_isdigit_boundary_after_nine" {
    // ':' (0x3A) 正好在 '9' (0x39) 之后
    let result = isdigit(b':' as c_int);
    assert_eq!(result, 0, "isdigit(':') 应返回 0");
});

test!("test_isdigit_eof" {
    let result = isdigit(EOF);
    assert_eq!(result, 0, "isdigit(EOF) 应返回 0");
});

test!("test_isdigit_control_characters" {
    for ch in 0x00..=0x1F {
        assert_eq!(isdigit(ch), 0, "isdigit(0x{:02X}) 应为 0", ch);
    }
    assert_eq!(isdigit(0x7F), 0, "isdigit(DEL) 应为 0");
});

test!("test_isdigit_extended_ascii" {
    for ch in [0x80u8, 0xC0, 0xFF].iter() {
        assert_eq!(isdigit(*ch as c_int), 0, "isdigit(0x{:02X}) 应为 0", ch);
    }
});

// ============================================================================
// isdigit_l 测试（locale 参数被忽略）
// ============================================================================

test!("test_isdigit_l_null_locale" {
    let result = isdigit_l(b'5' as c_int, core::ptr::null_mut());
    assert_ne!(result, 0, "isdigit_l('5', NULL) 应返回非零值");
});

test!("test_isdigit_l_non_digit" {
    let result = isdigit_l(b'a' as c_int, core::ptr::null_mut());
    assert_eq!(result, 0, "isdigit_l('a', NULL) 应返回 0");
});

test!("test_isdigit_l_eof" {
    let result = isdigit_l(EOF, core::ptr::null_mut());
    assert_eq!(result, 0, "isdigit_l(EOF, NULL) 应返回 0");
});

test!("test_isdigit_l_consistency_with_isdigit" {
    // isdigit_l 应与 isdigit 行为完全一致
    for ch in 0x00..=0xFF {
        let result_l = isdigit_l(ch, core::ptr::null_mut());
        let result = isdigit(ch);
        assert_eq!(
            result_l, result,
            "isdigit_l(0x{:02X}) = {} 应与 isdigit = {} 一致",
            ch, result_l, result
        );
    }
});

// ============================================================================
// 不变量验证
// ============================================================================

test!("test_isdigit_idempotent" {
    for ch in 0x00..=0xFF {
        let r1 = isdigit(ch);
        let r2 = isdigit(ch);
        assert_eq!(r1, r2, "isdigit(0x{:02X}) 多次调用应返回相同结果", ch);
    }
});

test!("test_isdigit_returns_only_zero_or_one" {
    for ch in 0x00..=0xFF {
        let result = isdigit(ch);
        assert!(result == 0 || result == 1, "isdigit(0x{:02X}) 返回值 {} 应为 0 或 1", ch, result);
    }
});

test!("test_isdigit_ten_digits_count" {
    let count: i32 = (0x00..=0xFF)
        .map(|ch| if isdigit(ch) != 0 { 1 } else { 0 })
        .sum();
    assert_eq!(count, 10, "恰好应有 10 个十进制数字，实际得到 {}", count);
});