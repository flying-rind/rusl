//! ispunct 集成测试 —— 从外部调用方角度验证 C ABI 兼容性。
//!
//! 本文件通过 `rusl::ctype::*` 导入符号，验证：
//! - 标点符号识别正确性（全部 32 个 ASCII 标点符号）
//! - 非标点符号返回 0（字母、数字、控制字符、空格）
//! - EOF (-1) 返回 0
//! - `ispunct_l` ABI 兼容性（locale 参数被忽略）
//!
//! ## 注意
//!
//! 当前 `ispunct` 函数体为 `todo!()`，因此所有调用该函数的测试均会 panic。
//! 测试结构、断言和覆盖范围已按 spec 设计完整，待实现完成后即生效。

use core::ffi::c_int;

use super::*;

// ============================================================================
// 常量
// ============================================================================

/// C 标准 EOF 值。
const EOF: c_int = -1;

// ============================================================================
// ispunct 基本功能测试
// ============================================================================

test!("test_ispunct_all_ascii_punctuation" {
    // 所有 ASCII 标点符号字符
    let punct_chars: [c_int; 32] = [
        b'!' as c_int, b'"' as c_int, b'#' as c_int, b'$' as c_int,
        b'%' as c_int, b'&' as c_int, b'\'' as c_int, b'(' as c_int,
        b')' as c_int, b'*' as c_int, b'+' as c_int, b',' as c_int,
        b'-' as c_int, b'.' as c_int, b'/' as c_int, b':' as c_int,
        b';' as c_int, b'<' as c_int, b'=' as c_int, b'>' as c_int,
        b'?' as c_int, b'@' as c_int, b'[' as c_int, b'\\' as c_int,
        b']' as c_int, b'^' as c_int, b'_' as c_int, b'`' as c_int,
        b'{' as c_int, b'|' as c_int, b'}' as c_int, b'~' as c_int,
    ];

    for &ch in &punct_chars {
        let result = ispunct(ch);
        assert_ne!(result, 0, "ispunct('{}' = 0x{:02X}) 应返回非零值", ch as u8 as char, ch);
    }
});

test!("test_ispunct_non_punctuation_lowercase" {
    // 所有小写字母不应是标点符号
    for ch in b'a'..=b'z' {
        let result = ispunct(ch as c_int);
        assert_eq!(result, 0, "ispunct('{}') 应为 0", ch as char);
    }
});

test!("test_ispunct_non_punctuation_uppercase" {
    // 所有大写字母不应是标点符号
    for ch in b'A'..=b'Z' {
        let result = ispunct(ch as c_int);
        assert_eq!(result, 0, "ispunct('{}') 应为 0", ch as char);
    }
});

test!("test_ispunct_non_punctuation_digits" {
    // 所有数字不应是标点符号
    for ch in b'0'..=b'9' {
        let result = ispunct(ch as c_int);
        assert_eq!(result, 0, "ispunct('{}') 应为 0", ch as char);
    }
});

// ============================================================================
// ispunct 边界与特殊值测试
// ============================================================================

test!("test_ispunct_eof" {
    let result = ispunct(EOF);
    assert_eq!(result, 0, "ispunct(EOF) 应返回 0");
});

test!("test_ispunct_space" {
    // 空格不是标点符号（空格是 printable 但通过 isgraph 排除）
    let result = ispunct(b' ' as c_int);
    assert_eq!(result, 0, "ispunct(' ') 应返回 0");
});

test!("test_ispunct_control_characters" {
    // 控制字符（0x00-0x1F）和 DEL（0x7F）不应是标点符号
    for ch in 0x00..=0x1F {
        let result = ispunct(ch);
        assert_eq!(result, 0, "ispunct(0x{:02X}) 应返回 0", ch);
    }
    assert_eq!(ispunct(0x7F), 0, "ispunct(DEL) 应返回 0");
});

test!("test_ispunct_boundary_before_exclamation" {
    // 空格(0x20) 是第一个可打印字符但在 isgraph 中不是图形字符
    assert_eq!(ispunct(0x20), 0, "ispunct(' ') 应返回 0");
    // '!'(0x21) 是第一个图形字符，也是第一个标点符号
    assert_ne!(ispunct(0x21), 0, "ispunct('!') 应返回非零值");
});

test!("test_ispunct_boundary_after_tilde" {
    // '~'(0x7E) 是最后一个标点符号
    assert_ne!(ispunct(0x7E), 0, "ispunct('~') 应返回非零值");
    // DEL(0x7F) 不是标点符号
    assert_eq!(ispunct(0x7F), 0, "ispunct(0x7F) 应返回 0");
});

test!("test_ispunct_specific_punctuation" {
    // 验证几个代表性标点符号
    assert_ne!(ispunct(b'!' as c_int), 0);
    assert_ne!(ispunct(b'@' as c_int), 0);
    assert_ne!(ispunct(b'[' as c_int), 0);
    assert_ne!(ispunct(b'`' as c_int), 0);
    assert_ne!(ispunct(b'{' as c_int), 0);
    assert_ne!(ispunct(b'~' as c_int), 0);
});

// ============================================================================
// ispunct_l 测试（locale 参数被忽略）
// ============================================================================

test!("test_ispunct_l_null_locale" {
    let result = ispunct_l(b'!' as c_int, core::ptr::null_mut());
    assert_ne!(result, 0, "ispunct_l('!', NULL) 应返回非零值");
});

test!("test_ispunct_l_non_punctuation" {
    let result = ispunct_l(b'A' as c_int, core::ptr::null_mut());
    assert_eq!(result, 0, "ispunct_l('A', NULL) 应返回 0");
});

test!("test_ispunct_l_eof" {
    let result = ispunct_l(EOF, core::ptr::null_mut());
    assert_eq!(result, 0, "ispunct_l(EOF, NULL) 应返回 0");
});

test!("test_ispunct_l_consistency_with_ispunct" {
    // ispunct_l 应与 ispunct 行为一致（遍历所有 ASCII 可打印字符验证）
    for ch in 0x00..=0x7F {
        let result_l = ispunct_l(ch, core::ptr::null_mut());
        let result = ispunct(ch);
        assert_eq!(
            result_l, result,
            "ispunct_l(0x{:02X}) = {} 应与 ispunct = {} 一致",
            ch, result_l, result
        );
    }
});

// ============================================================================
// 不变量验证
// ============================================================================

test!("test_ispunct_idempotent" {
    // ispunct 是纯函数，多次调用应返回相同结果
    for ch in 0x00..=0x7F {
        let r1 = ispunct(ch);
        let r2 = ispunct(ch);
        let r3 = ispunct(ch);
        assert_eq!(r1, r2);
        assert_eq!(r2, r3);
    }
});

test!("test_ispunct_returns_only_zero_or_one" {
    // musl 中 ispunct 仅返回 0 或 1
    for ch in 0x00..=0x7F {
        let result = ispunct(ch);
        assert!(result == 0 || result == 1, "ispunct(0x{:02X}) 返回值 {} 应为 0 或 1", ch, result);
    }
});