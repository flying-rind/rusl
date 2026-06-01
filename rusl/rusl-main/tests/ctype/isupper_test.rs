//! isupper 集成测试 —— 从外部调用方角度验证 C ABI 兼容性。
//!
//! 本文件通过 `rusl::ctype::*` 导入符号，验证：
//! - 全部 26 个大写字母 ('A'-'Z') 识别正确性
//! - 非大写字返回 0
//! - EOF (-1) 返回 0
//! - `isupper_l` ABI 兼容性（locale 参数被忽略）
//!
//! ## 注意
//!
//! 当前 `isupper` 函数体为 `todo!()`，因此所有调用该函数的测试均会 panic。
//! 测试结构、断言和覆盖范围已按 spec 设计完整，待实现完成后即生效。

use core::ffi::c_int;

use super::*;

// ============================================================================
// 常量
// ============================================================================

/// C 标准 EOF 值。
const EOF: c_int = -1;

/// 第一个大写字母 'A' 的整数值。
const CHAR_A: c_int = b'A' as c_int;

/// 最后一个大写字母 'Z' 的整数值。
const CHAR_Z: c_int = b'Z' as c_int;

// ============================================================================
// isupper 基本功能测试
// ============================================================================

test!("test_isupper_all_uppercase_letters" {
    for ch in CHAR_A..=CHAR_Z {
        let result = isupper(ch);
        assert_ne!(result, 0, "isupper('{}') 应返回非零值", ch as u8 as char);
    }
});

test!("test_isupper_lowercase_letters" {
    for ch in b'a'..=b'z' {
        let result = isupper(ch as c_int);
        assert_eq!(result, 0, "isupper('{}') 应为 0", ch as char);
    }
});

test!("test_isupper_digits" {
    for ch in b'0'..=b'9' {
        let result = isupper(ch as c_int);
        assert_eq!(result, 0, "isupper('{}') 应为 0", ch as char);
    }
});

test!("test_isupper_punctuation" {
    assert_eq!(isupper(b'!' as c_int), 0);
    assert_eq!(isupper(b'@' as c_int), 0);
    assert_eq!(isupper(b'[' as c_int), 0);
    assert_eq!(isupper(b'`' as c_int), 0);
    assert_eq!(isupper(b'{' as c_int), 0);
});

test!("test_isupper_control_characters" {
    for ch in 0x00..=0x1F {
        assert_eq!(isupper(ch), 0, "isupper(0x{:02X}) 应为 0", ch);
    }
    assert_eq!(isupper(0x7F), 0, "isupper(DEL) 应为 0");
});

// ============================================================================
// isupper 边界测试
// ============================================================================

test!("test_isupper_boundary_before_a" {
    // '@'(0x40) 正好在 'A'(0x41) 之前
    let result = isupper(0x40);
    assert_eq!(result, 0, "isupper('@') 应返回 0");
});

test!("test_isupper_boundary_first_uppercase" {
    // 'A'(0x41) 是第一个大写字母
    let result = isupper(CHAR_A);
    assert_ne!(result, 0, "isupper('A') 应返回非零值");
});

test!("test_isupper_boundary_last_uppercase" {
    // 'Z'(0x5A) 是最后一个大写字母
    let result = isupper(CHAR_Z);
    assert_ne!(result, 0, "isupper('Z') 应返回非零值");
});

test!("test_isupper_boundary_after_z" {
    // '['(0x5B) 正好在 'Z'(0x5A) 之后
    let result = isupper(0x5B);
    assert_eq!(result, 0, "isupper('[') 应返回 0");
});

test!("test_isupper_eof" {
    let result = isupper(EOF);
    assert_eq!(result, 0, "isupper(EOF) 应返回 0");
});

test!("test_isupper_extended_ascii" {
    // 扩展 ASCII (> 0x7F) 不应是大写字母
    for ch in [0x80u8, 0xC0, 0xFF].iter() {
        assert_eq!(isupper(*ch as c_int), 0, "isupper(0x{:02X}) 应为 0", ch);
    }
});

// ============================================================================
// isupper_l 测试（locale 参数被忽略）
// ============================================================================

test!("test_isupper_l_null_locale" {
    let result = isupper_l(b'A' as c_int, core::ptr::null_mut());
    assert_ne!(result, 0, "isupper_l('A', NULL) 应返回非零值");
});

test!("test_isupper_l_lowercase" {
    let result = isupper_l(b'a' as c_int, core::ptr::null_mut());
    assert_eq!(result, 0, "isupper_l('a', NULL) 应返回 0");
});

test!("test_isupper_l_eof" {
    let result = isupper_l(EOF, core::ptr::null_mut());
    assert_eq!(result, 0, "isupper_l(EOF, NULL) 应返回 0");
});

test!("test_isupper_l_consistency_with_isupper" {
    // isupper_l 应与 isupper 行为完全一致
    for ch in 0x00..=0xFF {
        let result_l = isupper_l(ch, core::ptr::null_mut());
        let result = isupper(ch);
        assert_eq!(
            result_l, result,
            "isupper_l(0x{:02X}) = {} 应与 isupper = {} 一致",
            ch, result_l, result
        );
    }
});

// ============================================================================
// 不变量验证
// ============================================================================

test!("test_isupper_idempotent" {
    for ch in 0x00..=0xFF {
        let r1 = isupper(ch);
        let r2 = isupper(ch);
        assert_eq!(r1, r2, "isupper(0x{:02X}) 多次调用应返回相同结果", ch);
    }
});

test!("test_isupper_returns_only_zero_or_one" {
    // musl 中 isupper 仅返回 0 或 1
    for ch in 0x00..=0xFF {
        let result = isupper(ch);
        assert!(result == 0 || result == 1, "isupper(0x{:02X}) 返回值 {} 应为 0 或 1", ch, result);
    }
});

test!("test_isupper_all_twenty_six_letters_count" {
    // 验证恰有 26 个大写字母
    let count: i32 = (CHAR_A..=CHAR_Z)
        .map(|ch| if isupper(ch) != 0 { 1 } else { 0 })
        .sum();
    assert_eq!(count, 26, "恰好应有 26 个大写字母，实际得到 {}", count);
});