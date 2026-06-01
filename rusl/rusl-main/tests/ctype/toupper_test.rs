//! toupper / toupper_l 集成测试
//!
//! 这些测试从外部 crate 的角度验证 toupper/toupper_l 函数的 C ABI 兼容性和语义正确性。
//! 集成测试链接到 rusl 静态库，模拟真实 C 代码的调用方式。


use super::*;

// ================================================================
// toupper 基本功能测试
// ================================================================

// 测试: 所有小写字母 'a'-'z' 正确转换为对应大写字母
test!("integration_test_toupper_lowercase_range" {
    unsafe {
        for ch in b'a'..=b'z' {
            let expected = (ch - 32) as c_int;
            assert_eq!(
                rusl::toupper(ch as c_int),
                expected,
                "toupper('{}') 应返回 '{}'",
                ch as char,
                expected as u8 as char
            );
        }
    }
});

// 测试: 大写字母 'A'-'Z' 保持不变
test!("integration_test_toupper_uppercase_unchanged" {
    unsafe {
        for ch in b'A'..=b'Z' {
            assert_eq!(
                rusl::toupper(ch as c_int),
                ch as c_int,
                "toupper('{}') 应保持不变",
                ch as char
            );
        }
    }
});

// 测试: 数字 '0'-'9' 保持不变
test!("integration_test_toupper_digits_unchanged" {
    unsafe {
        for ch in b'0'..=b'9' {
            assert_eq!(
                rusl::toupper(ch as c_int),
                ch as c_int,
                "toupper('{}') 应保持不变",
                ch as char
            );
        }
    }
});

// 测试: 边界值 '`' (a-1) 保持不变
test!("integration_test_toupper_boundary_below_a" {
    unsafe {
        assert_eq!(rusl::toupper(b'`' as c_int), b'`' as c_int);
    }
});

// 测试: 边界值 'a' 正确转换
test!("integration_test_toupper_boundary_a" {
    unsafe {
        assert_eq!(rusl::toupper(b'a' as c_int), b'A' as c_int);
    }
});

// 测试: 边界值 'z' 正确转换
test!("integration_test_toupper_boundary_z" {
    unsafe {
        assert_eq!(rusl::toupper(b'z' as c_int), b'Z' as c_int);
    }
});

// 测试: 边界值 '{' (z+1) 保持不变
test!("integration_test_toupper_boundary_above_z" {
    unsafe {
        assert_eq!(rusl::toupper(b'{' as c_int), b'{' as c_int);
    }
});

// 测试: EOF (-1) 保持不变
test!("integration_test_toupper_eof_unchanged" {
    unsafe {
        let eof: c_int = -1;
        assert_eq!(rusl::toupper(eof), eof);
    }
});

// 测试: null 字符 (0) 保持不变
test!("integration_test_toupper_null_char" {
    unsafe {
        assert_eq!(rusl::toupper(0), 0);
    }
});

// 测试: 最大值 0xFF 保持不变
test!("integration_test_toupper_max_unsigned_char" {
    unsafe {
        assert_eq!(rusl::toupper(0xFF), 0xFF);
    }
});

// ================================================================
// toupper_l 测试
// ================================================================

// 测试: toupper_l 与 toupper 对大小写字母行为一致
test!("integration_test_toupper_l_lowercase" {
    unsafe {
        let null_locale: *mut c_void = core::ptr::null_mut();
        for ch in b'a'..=b'z' {
            let expected = (ch - 32) as c_int;
            assert_eq!(
                rusl::toupper_l(ch as c_int, null_locale),
                expected,
                "toupper_l('{}', null) 应返回 '{}'",
                ch as char,
                expected as u8 as char
            );
        }
    }
});

// 测试: toupper_l 对非小写字符保持不变
test!("integration_test_toupper_l_non_lowercase" {
    unsafe {
        let null_locale: *mut c_void = core::ptr::null_mut();
        for ch in b'A'..=b'Z' {
            assert_eq!(
                rusl::toupper_l(ch as c_int, null_locale),
                ch as c_int,
                "toupper_l('{}', null) 应保持不变",
                ch as char
            );
        }
    }
});

// 测试: toupper_l 对 EOF 保持不变
test!("integration_test_toupper_l_eof" {
    unsafe {
        let null_locale: *mut c_void = core::ptr::null_mut();
        let eof: c_int = -1;
        assert_eq!(rusl::toupper_l(eof, null_locale), eof);
    }
});

// 测试: toupper_l 忽略 locale 参数（传入非空指针验证）
test!("integration_test_toupper_l_ignores_locale" {
    unsafe {
        let dummy: c_int = 0xBEEF;
        let dummy_ptr: *mut c_void = &dummy as *const c_int as *mut c_void;
        let result_null = rusl::toupper_l(b'a' as c_int, core::ptr::null_mut());
        let result_dummy = rusl::toupper_l(b'a' as c_int, dummy_ptr);
        assert_eq!(result_null, result_dummy, "toupper_l 应忽略 locale 参数");
    }
});