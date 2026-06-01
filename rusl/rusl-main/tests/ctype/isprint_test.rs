//! isprint 集成测试 —— 从外部调用方角度验证 C ABI 兼容性。
//!
//! 本文件通过 `rusl::ctype::*` 导入符号，验证：
//! - 全部 95 个可打印字符 (0x20-0x7E, 含空格) 识别正确性
//! - 控制字符 (0x00-0x1F) 和 DEL (0x7F) 返回 0
//! - EOF (-1) 返回 0
//! - `isprint_l` ABI 兼容性（locale 参数被忽略）
//!
//! ## 注意
//!
//! 当前 `isprint` 函数体为 `todo!()`，因此所有调用该函数的测试均会 panic。
//! 测试结构、断言和覆盖范围已按 spec 设计完整，待实现完成后即生效。

use core::ffi::c_int;

use super::*;

// ============================================================================
// 常量
// ============================================================================

/// C 标准 EOF 值。
const EOF: c_int = -1;

/// 第一个可打印字符（空格）的整数值。
const PRINT_MIN: c_int = 0x20;

/// 最后一个可打印字符 '~' 的整数值。
const PRINT_MAX: c_int = 0x7E;

// ============================================================================
// isprint 基本功能测试
// ============================================================================

test!("test_isprint_all_printable_characters" {
    // isprint 应接受 0x20-0x7E 范围内的所有字符
    for ch in PRINT_MIN..=PRINT_MAX {
        let result = isprint(ch);
        assert_ne!(result, 0, "isprint(0x{:02X}) 应返回非零值", ch);
    }
});

test!("test_isprint_space" {
    // 空格 (0x20) 是可打印字符
    let result = isprint(PRINT_MIN);
    assert_ne!(result, 0, "isprint(' ') 应返回非零值");
});

test!("test_isprint_digits" {
    for ch in b'0'..=b'9' {
        assert_ne!(isprint(ch as c_int), 0, "isprint('{}') 应返回非零值", ch as char);
    }
});

test!("test_isprint_uppercase" {
    for ch in b'A'..=b'Z' {
        assert_ne!(isprint(ch as c_int), 0, "isprint('{}') 应返回非零值", ch as char);
    }
});

test!("test_isprint_lowercase" {
    for ch in b'a'..=b'z' {
        assert_ne!(isprint(ch as c_int), 0, "isprint('{}') 应返回非零值", ch as char);
    }
});

test!("test_isprint_punctuation" {
    assert_ne!(isprint(b'!' as c_int), 0);
    assert_ne!(isprint(b'?' as c_int), 0);
    assert_ne!(isprint(b'~' as c_int), 0);
});

// ============================================================================
// isprint 边界与负例测试
// ============================================================================

test!("test_isprint_boundary_before_space" {
    // 0x1F (US) 正好在空格 (0x20) 之前
    let result = isprint(0x1F);
    assert_eq!(result, 0, "isprint(0x1F) 应返回 0（位于空格之前）");
});

test!("test_isprint_boundary_first_printable" {
    let result = isprint(PRINT_MIN);
    assert_ne!(result, 0, "isprint(0x20) 应返回非零值");
});

test!("test_isprint_boundary_last_printable" {
    let result = isprint(PRINT_MAX);
    assert_ne!(result, 0, "isprint(0x7E) 应返回非零值");
});

test!("test_isprint_boundary_after_tilde" {
    assert_eq!(isprint(0x7F), 0, "isprint(0x7F) 应返回 0（位于 '~' 之后）");
});

test!("test_isprint_control_characters" {
    // C0 控制字符 (0x00-0x1F) 都不可打印
    for ch in 0x00..=0x1F {
        assert_eq!(isprint(ch), 0, "isprint(0x{:02X}) 应为 0", ch);
    }
});

test!("test_isprint_del" {
    assert_eq!(isprint(0x7F), 0, "isprint(DEL) 应为 0");
});

test!("test_isprint_eof" {
    let result = isprint(EOF);
    assert_eq!(result, 0, "isprint(EOF) 应返回 0");
});

test!("test_isprint_extended_ascii" {
    for ch in [0x80u8, 0xC0, 0xFF].iter() {
        assert_eq!(isprint(*ch as c_int), 0, "isprint(0x{:02X}) 应为 0", ch);
    }
});

// ============================================================================
// isprint vs isgraph 关系测试
// ============================================================================

test!("test_isprint_is_graph_or_space" {
    // isprint(c) = isgraph(c) || c == ' '
    // 注意: 这需要 isgraph 函数，此处仅验证 isprint 包含空格
    assert_ne!(isprint(b' ' as c_int), 0, "isprint 应包含空格");
    assert_ne!(isprint(b'A' as c_int), 0, "isprint 应包含 'A'");
});

// ============================================================================
// isprint_l 测试（locale 参数被忽略）
// ============================================================================

test!("test_isprint_l_null_locale" {
    let result = isprint_l(b'A' as c_int, core::ptr::null_mut());
    assert_ne!(result, 0, "isprint_l('A', NULL) 应返回非零值");
});

test!("test_isprint_l_space" {
    let result = isprint_l(b' ' as c_int, core::ptr::null_mut());
    assert_ne!(result, 0, "isprint_l(' ', NULL) 应返回非零值");
});

test!("test_isprint_l_control" {
    let result = isprint_l(0x1F, core::ptr::null_mut());
    assert_eq!(result, 0, "isprint_l(0x1F, NULL) 应返回 0");
});

test!("test_isprint_l_eof" {
    let result = isprint_l(EOF, core::ptr::null_mut());
    assert_eq!(result, 0, "isprint_l(EOF, NULL) 应返回 0");
});

test!("test_isprint_l_consistency_with_isprint" {
    // isprint_l 应与 isprint 行为完全一致
    for ch in 0x00..=0xFF {
        let result_l = isprint_l(ch, core::ptr::null_mut());
        let result = isprint(ch);
        assert_eq!(
            result_l, result,
            "isprint_l(0x{:02X}) = {} 应与 isprint = {} 一致",
            ch, result_l, result
        );
    }
});

// ============================================================================
// 不变量验证
// ============================================================================

test!("test_isprint_idempotent" {
    for ch in 0x00..=0xFF {
        let r1 = isprint(ch);
        let r2 = isprint(ch);
        assert_eq!(r1, r2, "isprint(0x{:02X}) 多次调用应返回相同结果", ch);
    }
});

test!("test_isprint_returns_only_zero_or_one" {
    for ch in 0x00..=0xFF {
        let result = isprint(ch);
        assert!(result == 0 || result == 1, "isprint(0x{:02X}) 返回值 {} 应为 0 或 1", ch, result);
    }
});

test!("test_isprint_ninety_five_printable_chars" {
    let count: i32 = (0x00..=0xFF)
        .map(|ch| if isprint(ch) != 0 { 1 } else { 0 })
        .sum();
    assert_eq!(count, 95, "恰好应有 95 个可打印字符 (0x20-0x7E)，实际得到 {}", count);
});