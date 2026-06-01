//! isspace 集成测试 —— 从外部调用方角度验证 C ABI 兼容性。
//!
//! 本文件通过 `rusl::ctype::*` 导入符号，验证：
//! - 全部 6 个 C 标准空白字符识别正确性
//! - 非空白字符返回 0
//! - EOF (-1) 返回 0
//! - `isspace_l` ABI 兼容性（locale 参数被忽略）
//!
//! ## 注意
//!
//! 当前 `isspace` 函数体为 `todo!()`，因此所有调用该函数的测试均会 panic。
//! 测试结构、断言和覆盖范围已按 spec 设计完整，待实现完成后即生效。

use core::ffi::c_int;

use super::*;

// ============================================================================
// 常量
// ============================================================================

/// C 标准 EOF 值。
const EOF: c_int = -1;

/// 六个 C 标准空白字符。
const WHITESPACE_CHARS: [c_int; 6] = [
    b' ' as c_int,   // 空格  (0x20)
    b'\t' as c_int,  // 水平制表符 (0x09)
    b'\n' as c_int,  // 换行符 (0x0A)
    0x0B,            // 垂直制表符 (0x0B)
    0x0C,            // 换页符 (0x0C)
    b'\r' as c_int,  // 回车符 (0x0D)
];

// ============================================================================
// isspace 基本功能测试
// ============================================================================

test!("test_isspace_all_whitespace_chars" {
    // 所有 6 个 C 标准空白字符都应返回非零值
    for &ch in &WHITESPACE_CHARS {
        let result = isspace(ch);
        assert_ne!(result, 0, "isspace(0x{:02X}) 应返回非零值", ch);
    }
});

test!("test_isspace_non_whitespace_letters" {
    for ch in b'A'..=b'Z' {
        let result = isspace(ch as c_int);
        assert_eq!(result, 0, "isspace('{}') 应为 0", ch as char);
    }
    for ch in b'a'..=b'z' {
        let result = isspace(ch as c_int);
        assert_eq!(result, 0, "isspace('{}') 应为 0", ch as char);
    }
});

test!("test_isspace_non_whitespace_digits" {
    for ch in b'0'..=b'9' {
        let result = isspace(ch as c_int);
        assert_eq!(result, 0, "isspace('{}') 应为 0", ch as char);
    }
});

test!("test_isspace_non_whitespace_punctuation" {
    // 标点符号不应是空白字符
    assert_eq!(isspace(b'!' as c_int), 0);
    assert_eq!(isspace(b'.' as c_int), 0);
    assert_eq!(isspace(b',' as c_int), 0);
});

// ============================================================================
// isspace 边界测试
// ============================================================================

test!("test_isspace_boundary_before_tab_range" {
    // '\t'(0x09) 是 C 标准空白区间起点
    // 0x08 (BS) 不在连续区间 [0x09, 0x0D] 内
    let result = isspace(0x08);
    assert_eq!(result, 0, "isspace(0x08 BS) 应返回 0");
});

test!("test_isspace_boundary_after_cr_range" {
    // '\r'(0x0D) 是 C 标准空白区间终点
    // 0x0E (SO) 不在连续区间内
    let result = isspace(0x0E);
    assert_eq!(result, 0, "isspace(0x0E SO) 应返回 0");
});

test!("test_isspace_boundary_in_tab_range" {
    // 验证区间内每个字符都被正确识别
    // '\t'(0x09) ~ '\r'(0x0D) 全部是空白字符
    for ch in 0x09..=0x0D {
        let result = isspace(ch);
        assert_ne!(result, 0, "isspace(0x{:02X}) 应返回非零值", ch);
    }
});

test!("test_isspace_space_is_whitespace" {
    // 空格(0x20) 是空白字符
    let result = isspace(b' ' as c_int);
    assert_ne!(result, 0, "isspace(' ') 应返回非零值");
});

test!("test_isspace_eof" {
    let result = isspace(EOF);
    assert_eq!(result, 0, "isspace(EOF) 应返回 0");
});

test!("test_isspace_high_bytes" {
    // 高位字符（>= 0x80）不应是空白字符
    for ch in [0x80, 0xA0, 0xFF].iter() {
        assert_eq!(isspace(*ch), 0, "isspace(0x{:02X}) 应为 0", ch);
    }
});

// ============================================================================
// isspace_l 测试（locale 参数被忽略）
// ============================================================================

test!("test_isspace_l_null_locale" {
    let result = isspace_l(b' ' as c_int, core::ptr::null_mut());
    assert_ne!(result, 0, "isspace_l(' ', NULL) 应返回非零值");
});

test!("test_isspace_l_tab" {
    let result = isspace_l(b'\t' as c_int, core::ptr::null_mut());
    assert_ne!(result, 0, "isspace_l('\\t', NULL) 应返回非零值");
});

test!("test_isspace_l_newline" {
    let result = isspace_l(b'\n' as c_int, core::ptr::null_mut());
    assert_ne!(result, 0, "isspace_l('\\n', NULL) 应返回非零值");
});

test!("test_isspace_l_non_whitespace" {
    let result = isspace_l(b'A' as c_int, core::ptr::null_mut());
    assert_eq!(result, 0, "isspace_l('A', NULL) 应返回 0");
});

test!("test_isspace_l_eof" {
    let result = isspace_l(EOF, core::ptr::null_mut());
    assert_eq!(result, 0, "isspace_l(EOF, NULL) 应返回 0");
});

test!("test_isspace_l_consistency_with_isspace" {
    // isspace_l 应与 isspace 行为完全一致
    for ch in 0x00..=0xFF {
        let result_l = isspace_l(ch, core::ptr::null_mut());
        let result = isspace(ch);
        assert_eq!(
            result_l, result,
            "isspace_l(0x{:02X}) = {} 应与 isspace = {} 一致",
            ch, result_l, result
        );
    }
});

// ============================================================================
// 不变量验证
// ============================================================================

test!("test_isspace_idempotent" {
    for ch in 0x00..=0xFF {
        let r1 = isspace(ch);
        let r2 = isspace(ch);
        assert_eq!(r1, r2, "isspace(0x{:02X}) 多次调用应返回相同结果", ch);
    }
});

test!("test_isspace_returns_only_zero_or_one" {
    // musl 中 isspace 仅返回 0 或 1
    for ch in 0x00..=0xFF {
        let result = isspace(ch);
        assert!(result == 0 || result == 1, "isspace(0x{:02X}) 返回值 {} 应为 0 或 1", ch, result);
    }
});

test!("test_isspace_all_six_posix_whitespace" {
    // POSIX.1-2001 规定 isspace 必须识别这六个字符
    let posix_whitespace = [
        (' ', 0x20),
        ('\t', 0x09),
        ('\n', 0x0A),
        ('\x0B', 0x0B), // vertical tab
        ('\x0C', 0x0C), // form feed
        ('\r', 0x0D),
    ];

    for &(_name, code) in &posix_whitespace {
        assert_ne!(isspace(code), 0, "isspace(0x{:02X}) 应按 POSIX 返回非零值", code);
    }
});