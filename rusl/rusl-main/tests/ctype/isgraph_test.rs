//! isgraph 集成测试 —— 从外部调用方角度验证 C ABI 兼容性。
//!
//! 本文件通过 `rusl::ctype::*` 导入符号，验证：
//! - 全部 94 个可打印图形字符 (0x21-0x7E) 识别正确性
//! - 空格 (0x20) 和控制字符返回 0
//! - EOF (-1) 返回 0
//! - `isgraph_l` ABI 兼容性（locale 参数被忽略）
//!
//! ## 注意
//!
//! 当前 `isgraph` 函数体为 `todo!()`，因此所有调用该函数的测试均会 panic。
//! 测试结构、断言和覆盖范围已按 spec 设计完整，待实现完成后即生效。

use core::ffi::c_int;
use super::*;


// ============================================================================
// 常量
// ============================================================================

/// C 标准 EOF 值。
const EOF: c_int = -1;

/// 第一个图形字符 '!' 的整数值。
const GRAPH_MIN: c_int = 0x21;

/// 最后一个图形字符 '~' 的整数值。
const GRAPH_MAX: c_int = 0x7E;

// ============================================================================
// isgraph 基本功能测试
// ============================================================================

test!("test_isgraph_all_graphic_characters" {
    // isgraph 应接受 0x21-0x7E 范围内的所有字符
    for ch in GRAPH_MIN..=GRAPH_MAX {
        let result = isgraph(ch);
        assert_ne!(result, 0, "isgraph(0x{:02X}) 应返回非零值", ch);
    }
});

test!("test_isgraph_digits" {
    for ch in b'0'..=b'9' {
        assert_ne!(isgraph(ch as c_int), 0, "isgraph('{}') 应返回非零值", ch as char);
    }
});

test!("test_isgraph_uppercase" {
    for ch in b'A'..=b'Z' {
        assert_ne!(isgraph(ch as c_int), 0, "isgraph('{}') 应返回非零值", ch as char);
    }
});

test!("test_isgraph_lowercase" {
    for ch in b'a'..=b'z' {
        assert_ne!(isgraph(ch as c_int), 0, "isgraph('{}') 应返回非零值", ch as char);
    }
});

test!("test_isgraph_punctuation" {
    assert_ne!(isgraph(b'!' as c_int), 0);
    assert_ne!(isgraph(b'?' as c_int), 0);
    assert_ne!(isgraph(b'@' as c_int), 0);
    assert_ne!(isgraph(b'[' as c_int), 0);
    assert_ne!(isgraph(b'`' as c_int), 0);
    assert_ne!(isgraph(b'{' as c_int), 0);
    assert_ne!(isgraph(b'~' as c_int), 0);
});

// ============================================================================
// isgraph 边界与负例测试
// ============================================================================

test!("test_isgraph_space_is_not_graph" {
    // 空格 (0x20) 不是图形字符
    let result = isgraph(b' ' as c_int);
    assert_eq!(result, 0, "isgraph(' ') 应返回 0");
});

test!("test_isgraph_boundary_before_exclamation" {
    assert_eq!(isgraph(0x20), 0, "isgraph(0x20) 应返回 0（位于 '!' 之前）");
});

test!("test_isgraph_boundary_first_graph" {
    assert_ne!(isgraph(GRAPH_MIN), 0, "isgraph('!') 应返回非零值");
});

test!("test_isgraph_boundary_last_graph" {
    assert_ne!(isgraph(GRAPH_MAX), 0, "isgraph('~') 应返回非零值");
});

test!("test_isgraph_boundary_after_tilde" {
    assert_eq!(isgraph(0x7F), 0, "isgraph(0x7F) 应返回 0（位于 '~' 之后）");
});

test!("test_isgraph_control_characters" {
    for ch in 0x00..=0x1F {
        assert_eq!(isgraph(ch), 0, "isgraph(0x{:02X}) 应为 0", ch);
    }
});

test!("test_isgraph_eof" {
    let result = isgraph(EOF);
    assert_eq!(result, 0, "isgraph(EOF) 应返回 0");
});

test!("test_isgraph_extended_ascii" {
    for ch in [0x80u8, 0xC0, 0xFF].iter() {
        assert_eq!(isgraph(*ch as c_int), 0, "isgraph(0x{:02X}) 应为 0", ch);
    }
});

// ============================================================================
// isgraph_l 测试（locale 参数被忽略）
// ============================================================================

test!("test_isgraph_l_null_locale" {
    let result = isgraph_l(b'@' as c_int, core::ptr::null_mut());
    assert_ne!(result, 0, "isgraph_l('@', NULL) 应返回非零值");
});

test!("test_isgraph_l_space" {
    let result = isgraph_l(b' ' as c_int, core::ptr::null_mut());
    assert_eq!(result, 0, "isgraph_l(' ', NULL) 应返回 0");
});

test!("test_isgraph_l_eof" {
    let result = isgraph_l(EOF, core::ptr::null_mut());
    assert_eq!(result, 0, "isgraph_l(EOF, NULL) 应返回 0");
});

test!("test_isgraph_l_consistency_with_isgraph" {
    // isgraph_l 应与 isgraph 行为完全一致
    for ch in 0x00..=0xFF {
        let result_l = isgraph_l(ch, core::ptr::null_mut());
        let result = isgraph(ch);
        assert_eq!(
            result_l, result,
            "isgraph_l(0x{:02X}) = {} 应与 isgraph = {} 一致",
            ch, result_l, result
        );
    }
});

// ============================================================================
// 不变量验证
// ============================================================================

test!("test_isgraph_idempotent" {
    for ch in 0x00..=0xFF {
        let r1 = isgraph(ch);
        let r2 = isgraph(ch);
        assert_eq!(r1, r2, "isgraph(0x{:02X}) 多次调用应返回相同结果", ch);
    }
});

test!("test_isgraph_returns_only_zero_or_one" {
    for ch in 0x00..=0xFF {
        let result = isgraph(ch);
        assert!(result == 0 || result == 1, "isgraph(0x{:02X}) 返回值 {} 应为 0 或 1", ch, result);
    }
});

test!("test_isgraph_ninety_four_graphic_chars" {
    let count: i32 = (0x00..=0xFF)
        .map(|ch| if isgraph(ch) != 0 { 1 } else { 0 })
        .sum();
    assert_eq!(count, 94, "恰好应有 94 个图形字符 (0x21-0x7E)，实际得到 {}", count);
});