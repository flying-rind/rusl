//! islower 集成测试 —— 从外部调用方角度验证 C ABI 兼容性。
//!
//! 本文件通过 `rusl::ctype::*` 导入符号，验证：
//! - 全部 26 个小写字母 ('a'-'z') 识别正确性
//! - 非小写字母返回 0
//! - EOF (-1) 返回 0
//! - `islower_l` ABI 兼容性（locale 参数被忽略）
//!
//! ## 注意
//!
//! 当前 `islower` 函数体为 `todo!()`，因此所有调用该函数的测试均会 panic。
//! 测试结构、断言和覆盖范围已按 spec 设计完整，待实现完成后即生效。

use core::ffi::c_int;
use super::*;

// ============================================================================
// 常量
// ============================================================================

/// C 标准 EOF 值。
const EOF: c_int = -1;

/// 第一个小写字母 'a' 的整数值。
const CHAR_A: c_int = b'a' as c_int;

/// 最后一个小写字母 'z' 的整数值。
const CHAR_Z: c_int = b'z' as c_int;

// ============================================================================
// islower 基本功能测试
// ============================================================================

test!("test_islower_all_lowercase_letters" {
    for ch in CHAR_A..=CHAR_Z {
        let result = islower(ch);
        assert_ne!(result, 0, "islower('{}') 应返回非零值", ch as u8 as char);
    }
});

test!("test_islower_uppercase_letters" {
    for ch in b'A'..=b'Z' {
        let result = islower(ch as c_int);
        assert_eq!(result, 0, "islower('{}') 应为 0", ch as char);
    }
});

test!("test_islower_digits" {
    for ch in b'0'..=b'9' {
        let result = islower(ch as c_int);
        assert_eq!(result, 0, "islower('{}') 应为 0", ch as char);
    }
});

test!("test_islower_punctuation" {
    assert_eq!(islower(b'!' as c_int), 0);
    assert_eq!(islower(b'@' as c_int), 0);
    assert_eq!(islower(b'[' as c_int), 0);
    assert_eq!(islower(b'`' as c_int), 0);
    assert_eq!(islower(b'{' as c_int), 0);
});

test!("test_islower_control_characters" {
    for ch in 0x00..=0x1F {
        assert_eq!(islower(ch), 0, "islower(0x{:02X}) 应为 0", ch);
    }
    assert_eq!(islower(0x7F), 0, "islower(DEL) 应为 0");
});

// ============================================================================
// islower 边界测试
// ============================================================================

test!("test_islower_boundary_before_a" {
    // '`' (0x60, backtick) 正好在 'a' (0x61) 之前
    let result = islower(0x60);
    assert_eq!(result, 0, "islower('`') 应返回 0");
});

test!("test_islower_boundary_first_lowercase" {
    // 'a' (0x61) 是第一个小写字母
    let result = islower(CHAR_A);
    assert_ne!(result, 0, "islower('a') 应返回非零值");
});

test!("test_islower_boundary_last_lowercase" {
    // 'z' (0x7A) 是最后一个小写字母
    let result = islower(CHAR_Z);
    assert_ne!(result, 0, "islower('z') 应返回非零值");
});

test!("test_islower_boundary_after_z" {
    // '{' (0x7B) 正好在 'z' (0x7A) 之后
    let result = islower(0x7B);
    assert_eq!(result, 0, "islower('{{') 应返回 0");
});

test!("test_islower_eof" {
    let result = islower(EOF);
    assert_eq!(result, 0, "islower(EOF) 应返回 0");
});

test!("test_islower_extended_ascii" {
    // 扩展 ASCII (> 0x7F) 不应是小写字母
    for ch in [0x80u8, 0xC0, 0xFF].iter() {
        assert_eq!(islower(*ch as c_int), 0, "islower(0x{:02X}) 应为 0", ch);
    }
});

// ============================================================================
// islower_l 测试（locale 参数被忽略）
// ============================================================================

test!("test_islower_l_null_locale" {
    let result = islower_l(b'a' as c_int, core::ptr::null_mut());
    assert_ne!(result, 0, "islower_l('a', NULL) 应返回非零值");
});

test!("test_islower_l_uppercase" {
    let result = islower_l(b'A' as c_int, core::ptr::null_mut());
    assert_eq!(result, 0, "islower_l('A', NULL) 应返回 0");
});

test!("test_islower_l_eof" {
    let result = islower_l(EOF, core::ptr::null_mut());
    assert_eq!(result, 0, "islower_l(EOF, NULL) 应返回 0");
});

test!("test_islower_l_consistency_with_islower" {
    // islower_l 应与 islower 行为完全一致
    for ch in 0x00..=0xFF {
        let result_l = islower_l(ch, core::ptr::null_mut());
        let result = islower(ch);
        assert_eq!(
            result_l, result,
            "islower_l(0x{:02X}) = {} 应与 islower = {} 一致",
            ch, result_l, result
        );
    }
});

// ============================================================================
// 不变量验证
// ============================================================================

test!("test_islower_idempotent" {
    for ch in 0x00..=0xFF {
        let r1 = islower(ch);
        let r2 = islower(ch);
        assert_eq!(r1, r2, "islower(0x{:02X}) 多次调用应返回相同结果", ch);
    }
});

test!("test_islower_returns_only_zero_or_one" {
    // musl 中 islower 仅返回 0 或 1
    for ch in 0x00..=0xFF {
        let result = islower(ch);
        assert!(result == 0 || result == 1, "islower(0x{:02X}) 返回值 {} 应为 0 或 1", ch, result);
    }
});

test!("test_islower_all_twenty_six_letters_count" {
    // 验证恰有 26 个小写字母
    let count: i32 = (0x00..=0xFF)
        .map(|ch| if islower(ch) != 0 { 1 } else { 0 })
        .sum();
    assert_eq!(count, 26, "恰好应有 26 个小写字母，实际得到 {}", count);
});