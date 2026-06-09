//! tolower / tolower_l 集成测试
//!
//! 这些测试从外部 crate 的角度验证 tolower/tolower_l 函数的 C ABI 兼容性和语义正确性。
//! 集成测试链接到 rusl 静态库，模拟真实 C 代码的调用方式。

use super::*;


// ================================================================
// tolower 基本功能测试
// ================================================================

// 测试: 所有大写字母 'A'-'Z' 正确转换为对应小写字母
test!("integration_test_tolower_uppercase_range" {
    {
        for ch in b'A'..=b'Z' {
            let expected = (ch + 32) as c_int;
            assert_eq!(
                rusl::tolower(ch as c_int),
                expected,
                "tolower('{}') 应返回 '{}'",
                ch as char,
                expected as u8 as char
            );
        }
    }
});

// 测试: 小写字母 'a'-'z' 保持不变
test!("integration_test_tolower_lowercase_unchanged" {
    {
        for ch in b'a'..=b'z' {
            assert_eq!(
                rusl::tolower(ch as c_int),
                ch as c_int,
                "tolower('{}') 应保持不变",
                ch as char
            );
        }
    }
});

// 测试: 数字 '0'-'9' 保持不变
test!("integration_test_tolower_digits_unchanged" {
    {
        for ch in b'0'..=b'9' {
            assert_eq!(
                rusl::tolower(ch as c_int),
                ch as c_int,
                "tolower('{}') 应保持不变",
                ch as char
            );
        }
    }
});

// 测试: 边界值 '@' (A-1) 保持不变
test!("integration_test_tolower_boundary_below_a" {
    {
        assert_eq!(rusl::tolower(b'@' as c_int), b'@' as c_int);
    }
});

// 测试: 边界值 'A' 正确转换
test!("integration_test_tolower_boundary_a" {
    {
        assert_eq!(rusl::tolower(b'A' as c_int), b'a' as c_int);
    }
});

// 测试: 边界值 'Z' 正确转换
test!("integration_test_tolower_boundary_z" {
    {
        assert_eq!(rusl::tolower(b'Z' as c_int), b'z' as c_int);
    }
});

// 测试: 边界值 '[' (Z+1) 保持不变
test!("integration_test_tolower_boundary_above_z" {
    {
        assert_eq!(rusl::tolower(b'[' as c_int), b'[' as c_int);
    }
});

// 测试: EOF (-1) 保持不变
test!("integration_test_tolower_eof_unchanged" {
    {
        let eof: c_int = -1;
        assert_eq!(rusl::tolower(eof), eof);
    }
});

// 测试: null 字符 (0) 保持不变
test!("integration_test_tolower_null_char" {
    {
        assert_eq!(rusl::tolower(0), 0);
    }
});

// 测试: 最大值 0xFF 保持不变
test!("integration_test_tolower_max_unsigned_char" {
    {
        assert_eq!(rusl::tolower(0xFF), 0xFF);
    }
});

// ================================================================
// tolower_l 测试
// ================================================================

// 测试: tolower_l 与 tolower 对大小写字母行为一致
test!("integration_test_tolower_l_uppercase" {
    {
        let null_locale: *mut c_void = core::ptr::null_mut();
        for ch in b'A'..=b'Z' {
            let expected = (ch + 32) as c_int;
            assert_eq!(
                rusl::tolower_l(ch as c_int, null_locale),
                expected,
                "tolower_l('{}', null) 应返回 '{}'",
                ch as char,
                expected as u8 as char
            );
        }
    }
});

// 测试: tolower_l 对非大写字符保持不变
test!("integration_test_tolower_l_non_uppercase" {
    {
        let null_locale: *mut c_void = core::ptr::null_mut();
        for ch in b'a'..=b'z' {
            assert_eq!(
                rusl::tolower_l(ch as c_int, null_locale),
                ch as c_int,
                "tolower_l('{}', null) 应保持不变",
                ch as char
            );
        }
    }
});

// 测试: tolower_l 对 EOF 保持不变
test!("integration_test_tolower_l_eof" {
    {
        let null_locale: *mut c_void = core::ptr::null_mut();
        let eof: c_int = -1;
        assert_eq!(rusl::tolower_l(eof, null_locale), eof);
    }
});

// 测试: tolower_l 忽略 locale 参数（传入非空指针验证）
test!("integration_test_tolower_l_ignores_locale" {
    {
        let dummy: c_int = 0xDEAD;
        let dummy_ptr: *mut c_void = &dummy as *const c_int as *mut c_void;
        let result_null = rusl::tolower_l(b'A' as c_int, core::ptr::null_mut());
        let result_dummy = rusl::tolower_l(b'A' as c_int, dummy_ptr);
        assert_eq!(result_null, result_dummy, "tolower_l 应忽略 locale 参数");
    }
});