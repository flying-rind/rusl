//! towlower / towupper 集成测试
//!
//! 这些测试从外部 crate 的角度验证 towlower/towupper 函数的 C ABI 兼容性和语义正确性。
//! 集成测试链接到 rusl 静态库，模拟真实 C 代码的调用方式。


use super::*;
// ================================================================
// towlower 基本功能测试
// ================================================================

// 测试: ASCII 大写字母 'A'-'Z' 正确转换为小写
test!("integration_test_towlower_ascii_uppercase_to_lowercase" {
    {
        for ch in b'A'..=b'Z' {
            let expected = (ch + 32) as wint_t;
            assert_eq!(
                towlower(ch as wint_t),
                expected,
                "towlower('{}') 应返回 '{}'",
                ch as char,
                expected as u8 as char
            );
        }
    }
});

// 测试: ASCII 小写字母 'a'-'z' 保持不变
test!("integration_test_towlower_ascii_lowercase_unchanged" {
    {
        for ch in b'a'..=b'z' {
            assert_eq!(
                towlower(ch as wint_t),
                ch as wint_t,
                "towlower('{}') 应保持不变",
                ch as char
            );
        }
    }
});

// 测试: 数字 '0'-'9' 保持不变
test!("integration_test_towlower_digits_unchanged" {
    {
        for ch in b'0'..=b'9' {
            assert_eq!(
                towlower(ch as wint_t),
                ch as wint_t,
                "towlower('{}') 应保持不变",
                ch as char
            );
        }
    }
});

// 测试: WEOF 保持不变
test!("integration_test_towlower_weof_unchanged" {
    {
        assert_eq!(towlower(WEOF), WEOF);
    }
});

// 测试: null 字符 (0) 保持不变
test!("integration_test_towlower_null_char" {
    {
        assert_eq!(towlower(0), 0);
    }
});

// 测试: Unicode Latin 扩展大写字母转换
// 例如 U+00C0 (LATIN CAPITAL LETTER A WITH GRAVE) -> U+00E0
test!("integration_test_towlower_latin_extended" {
    {
        // LATIN CAPITAL LETTER A WITH GRAVE (U+00C0) -> U+00E0
        assert_eq!(towlower(0x00C0), 0x00E0,
            "towlower(U+00C0) 应返回 U+00E0");
        // LATIN CAPITAL LETTER ETH (U+00D0) -> U+00F0
        assert_eq!(towlower(0x00D0), 0x00F0,
            "towlower(U+00D0) 应返回 U+00F0");
    }
});

// ================================================================
// towupper 基本功能测试
// ================================================================

// 测试: ASCII 小写字母 'a'-'z' 正确转换为大写
test!("integration_test_towupper_ascii_lowercase_to_uppercase" {
    {
        for ch in b'a'..=b'z' {
            let expected = (ch - 32) as wint_t;
            assert_eq!(
                towupper(ch as wint_t),
                expected,
                "towupper('{}') 应返回 '{}'",
                ch as char,
                expected as u8 as char
            );
        }
    }
});

// 测试: ASCII 大写字母 'A'-'Z' 保持不变
test!("integration_test_towupper_ascii_uppercase_unchanged" {
    {
        for ch in b'A'..=b'Z' {
            assert_eq!(
                towupper(ch as wint_t),
                ch as wint_t,
                "towupper('{}') 应保持不变",
                ch as char
            );
        }
    }
});

// 测试: WEOF 保持不变
test!("integration_test_towupper_weof_unchanged" {
    {
        assert_eq!(towupper(WEOF), WEOF);
    }
});

// 测试: null 字符 (0) 保持不变
test!("integration_test_towupper_null_char" {
    {
        assert_eq!(towupper(0), 0);
    }
});

// 测试: Unicode Latin 扩展小写字母转换
// 例如 U+00E0 (LATIN SMALL LETTER A WITH GRAVE) -> U+00C0
test!("integration_test_towupper_latin_extended" {
    {
        // LATIN SMALL LETTER A WITH GRAVE (U+00E0) -> U+00C0
        assert_eq!(towupper(0x00E0), 0x00C0,
            "towupper(U+00E0) 应返回 U+00C0");
        // LATIN SMALL LETTER ETH (U+00F0) -> U+00D0
        assert_eq!(towupper(0x00F0), 0x00D0,
            "towupper(U+00F0) 应返回 U+00D0");
    }
});

// ================================================================
// towlower_l / towupper_l 测试
// ================================================================

// 测试: towlower_l 与 towlower 对 ASCII 大写字母行为一致
test!("integration_test_towlower_l_uppercase" {
    {
        let null_locale: *mut c_void = core::ptr::null_mut();
        for ch in b'A'..=b'Z' {
            let expected = (ch + 32) as wint_t;
            assert_eq!(
                towlower_l(ch as wint_t, null_locale),
                expected,
                "towlower_l('{}', null) 应返回 '{}'",
                ch as char,
                expected as u8 as char
            );
        }
    }
});

// 测试: towlower_l 对 WEOF 保持不变
test!("integration_test_towlower_l_weof" {
    {
        let null_locale: *mut c_void = core::ptr::null_mut();
        assert_eq!(towlower_l(WEOF, null_locale), WEOF);
    }
});

// 测试: towupper_l 与 towupper 对 ASCII 小写字母行为一致
test!("integration_test_towupper_l_lowercase" {
    {
        let null_locale: *mut c_void = core::ptr::null_mut();
        for ch in b'a'..=b'z' {
            let expected = (ch - 32) as wint_t;
            assert_eq!(
                towupper_l(ch as wint_t, null_locale),
                expected,
                "towupper_l('{}', null) 应返回 '{}'",
                ch as char,
                expected as u8 as char
            );
        }
    }
});

// 测试: towupper_l 对 WEOF 保持不变
test!("integration_test_towupper_l_weof" {
    {
        let null_locale: *mut c_void = core::ptr::null_mut();
        assert_eq!(towupper_l(WEOF, null_locale), WEOF);
    }
});

// 测试: towlower_l 忽略 locale 参数
test!("integration_test_towlower_l_ignores_locale" {
    {
        let dummy: u32 = 0xDEAD;
        let dummy_ptr: *mut c_void = &dummy as *const u32 as *mut c_void;
        let result_null = towlower_l(b'A' as wint_t, core::ptr::null_mut());
        let result_dummy = towlower_l(b'A' as wint_t, dummy_ptr);
        assert_eq!(result_null, result_dummy, "towlower_l 应忽略 locale 参数");
    }
});

// 测试: towupper_l 忽略 locale 参数
test!("integration_test_towupper_l_ignores_locale" {
    {
        let dummy: u32 = 0xBEEF;
        let dummy_ptr: *mut c_void = &dummy as *const u32 as *mut c_void;
        let result_null = towupper_l(b'a' as wint_t, core::ptr::null_mut());
        let result_dummy = towupper_l(b'a' as wint_t, dummy_ptr);
        assert_eq!(result_null, result_dummy, "towupper_l 应忽略 locale 参数");
    }
});

// ================================================================
// Idempotent 测试
// ================================================================

// 测试: towlower 是幂等的（对已经是小写的字符再次调用不变）
test!("integration_test_towlower_idempotent" {
    {
        for ch in b'A'..=b'Z' {
            let lower = towlower(ch as wint_t);
            let lower_again = towlower(lower);
            assert_eq!(lower_again, lower,
                "towlower(towlower('{}')) 应等于 towlower('{}')",
                ch as char, ch as char);
        }
    }
});

// 测试: towupper 是幂等的
test!("integration_test_towupper_idempotent" {
    {
        for ch in b'a'..=b'z' {
            let upper = towupper(ch as wint_t);
            let upper_again = towupper(upper);
            assert_eq!(upper_again, upper,
                "towupper(towupper('{}')) 应等于 towupper('{}')",
                ch as char, ch as char);
        }
    }
});

// 测试: towlower 后 towupper 应还原大写字母
test!("integration_test_towlower_towupper_roundtrip" {
    {
        for ch in b'A'..=b'Z' {
            let lower = towlower(ch as wint_t);
            let back = towupper(lower);
            assert_eq!(back, ch as wint_t,
                "towupper(towlower('{}')) 应返回 '{}'",
                ch as char, ch as char);
        }
    }
});

// 测试: CJK Extension B 及以上码点无大小写
test!("integration_test_towlower_towupper_cjk_ext_b" {
    {
        // CJK Extension B 起始: U+20000
        let cjk: wint_t = 0x20000;
        assert_eq!(towlower(cjk), cjk,
            "towlower(U+20000) 应返回原值");
        assert_eq!(towupper(cjk), cjk,
            "towupper(U+20000) 应返回原值");
    }
});