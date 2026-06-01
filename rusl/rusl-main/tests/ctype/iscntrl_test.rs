//! iscntrl 集成测试 —— 从外部调用方角度验证 C ABI 兼容性。
//!
//! 本文件通过 `rusl::ctype::*` 导入符号，验证：
//! - 控制字符 (0x00-0x1F) 和 DEL (0x7F) 识别正确性
//! - 非控制字符返回 0
//! - EOF (-1) 返回 0
//! - `iscntrl_l` ABI 兼容性（locale 参数被忽略）
//!
//! ## 注意
//!
//! 当前 `iscntrl` 函数体为 `todo!()`，因此所有调用该函数的测试均会 panic。
//! 测试结构、断言和覆盖范围已按 spec 设计完整，待实现完成后即生效。

use core::ffi::c_int;

use super::*;

// ============================================================================
// 常量
// ============================================================================

/// C 标准 EOF 值。
const EOF: c_int = -1;

// ============================================================================
// iscntrl 基本功能测试
// ============================================================================

test!("test_iscntrl_c0_range" {
    // C0 控制字符: 0x00-0x1F (共 32 个)
    for ch in 0x00..=0x1F {
        let result = iscntrl(ch);
        assert_ne!(result, 0, "iscntrl(0x{:02X}) 应返回非零值", ch);
    }
});

test!("test_iscntrl_nul" {
    let result = iscntrl(0x00);
    assert_ne!(result, 0, "iscntrl(NUL) 应返回非零值");
});

test!("test_iscntrl_bell" {
    // BEL = 0x07
    let result = iscntrl(0x07);
    assert_ne!(result, 0, "iscntrl(BEL) 应返回非零值");
});

test!("test_iscntrl_escape" {
    // ESC = 0x1B
    let result = iscntrl(0x1B);
    assert_ne!(result, 0, "iscntrl(ESC) 应返回非零值");
});

test!("test_iscntrl_unit_separator" {
    // US = 0x1F, C0 范围最后一个
    let result = iscntrl(0x1F);
    assert_ne!(result, 0, "iscntrl(US) 应返回非零值");
});

test!("test_iscntrl_del" {
    let result = iscntrl(0x7F);
    assert_ne!(result, 0, "iscntrl(DEL) 应返回非零值");
});

test!("test_iscntrl_non_control_characters" {
    // 字母、数字、标点都不是控制字符
    assert_eq!(iscntrl(b'a' as c_int), 0);
    assert_eq!(iscntrl(b'Z' as c_int), 0);
    assert_eq!(iscntrl(b'5' as c_int), 0);
    assert_eq!(iscntrl(b'!' as c_int), 0);
    assert_eq!(iscntrl(b' ' as c_int), 0, "iscntrl(' ') 应为 0");
});

// ============================================================================
// iscntrl 边界测试
// ============================================================================

test!("test_iscntrl_boundary_before_c0" {
    // 负值（超出 unsigned char 范围）
    // 注意: c0 从 0x00 开始，没有"之前"的正整数
    // 但负数如 -1(EOF) 不应是控制字符
    let result = iscntrl(EOF);
    assert_eq!(result, 0, "iscntrl(EOF) 应返回 0");
});

test!("test_iscntrl_boundary_after_c0" {
    // 0x20 (空格) 正好在 C0 范围之后
    let result = iscntrl(0x20);
    assert_eq!(result, 0, "iscntrl(' ') 应返回 0（位于 C0 范围之后）");
});

test!("test_iscntrl_boundary_before_del" {
    // 0x7E ('~') 正好在 DEL (0x7F) 之前
    let result = iscntrl(0x7E);
    assert_eq!(result, 0, "iscntrl('~') 应返回 0（位于 DEL 之前）");
});

test!("test_iscntrl_boundary_del" {
    let result = iscntrl(0x7F);
    assert_ne!(result, 0, "iscntrl(DEL) 应返回非零值");
});

test!("test_iscntrl_boundary_after_del" {
    // 0x80 在 DEL 之后
    let result = iscntrl(0x80);
    assert_eq!(result, 0, "iscntrl(0x80) 应返回 0（位于 DEL 之后）");
});

test!("test_iscntrl_eof" {
    let result = iscntrl(EOF);
    assert_eq!(result, 0, "iscntrl(EOF) 应返回 0");
});

test!("test_iscntrl_extended_ascii" {
    // 扩展 ASCII (> 0x7F) 不应是控制字符
    for ch in [0x80u8, 0xC0, 0xFF].iter() {
        assert_eq!(iscntrl(*ch as c_int), 0, "iscntrl(0x{:02X}) 应为 0", ch);
    }
});

// ============================================================================
// iscntrl_l 测试（locale 参数被忽略）
// ============================================================================

test!("test_iscntrl_l_null_locale" {
    let result = iscntrl_l(0x1F, core::ptr::null_mut());
    assert_ne!(result, 0, "iscntrl_l(0x1F, NULL) 应返回非零值");
});

test!("test_iscntrl_l_del" {
    let result = iscntrl_l(0x7F, core::ptr::null_mut());
    assert_ne!(result, 0, "iscntrl_l(DEL, NULL) 应返回非零值");
});

test!("test_iscntrl_l_non_control" {
    let result = iscntrl_l(b'A' as c_int, core::ptr::null_mut());
    assert_eq!(result, 0, "iscntrl_l('A', NULL) 应返回 0");
});

test!("test_iscntrl_l_eof" {
    let result = iscntrl_l(EOF, core::ptr::null_mut());
    assert_eq!(result, 0, "iscntrl_l(EOF, NULL) 应返回 0");
});

test!("test_iscntrl_l_consistency_with_iscntrl" {
    // iscntrl_l 应与 iscntrl 行为完全一致
    for ch in 0x00..=0xFF {
        let result_l = iscntrl_l(ch, core::ptr::null_mut());
        let result = iscntrl(ch);
        assert_eq!(
            result_l, result,
            "iscntrl_l(0x{:02X}) = {} 应与 iscntrl = {} 一致",
            ch, result_l, result
        );
    }
});

// ============================================================================
// 不变量验证
// ============================================================================

test!("test_iscntrl_idempotent" {
    // iscntrl 是纯函数，多次调用应返回相同结果
    for ch in 0x00..=0xFF {
        let r1 = iscntrl(ch);
        let r2 = iscntrl(ch);
        assert_eq!(r1, r2, "iscntrl(0x{:02X}) 多次调用应返回相同结果", ch);
    }
});

test!("test_iscntrl_returns_only_zero_or_one" {
    // musl 中 iscntrl 仅返回 0 或 1
    for ch in 0x00..=0xFF {
        let result = iscntrl(ch);
        assert!(result == 0 || result == 1, "iscntrl(0x{:02X}) 返回值 {} 应为 0 或 1", ch, result);
    }
});

test!("test_iscntrl_thirty_three_control_chars" {
    // 验证恰有 33 个控制字符：C0 (0x00-0x1F, 32 个) + DEL (0x7F, 1 个)
    let count: i32 = (0x00..=0xFF)
        .map(|ch| if iscntrl(ch) != 0 { 1 } else { 0 })
        .sum();
    assert_eq!(count, 33, "恰好应有 33 个控制字符，实际得到 {}", count);
});