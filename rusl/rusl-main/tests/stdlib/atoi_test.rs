/// 模块: atoi_test
/// `atoi` 集成测试

use core::ffi::{c_char};
use super::imports::{atoi, atol, atoll};

// [removed alloc import: // [removed alloc import: use alloc::ffi::CString;]]
use rusl_core::test;

// ---- atoi 测试 ----

test!("test_atoi_positive" {
    // 测试 atoi 基本正数解析。
    unsafe {
        let s = b"123".as_ptr() as *const c_char;
        assert_eq!(atoi(s), 123);
    }
});

test!("test_atoi_negative" {
    // 测试 atoi 负数解析。
    unsafe {
        let s = b"-456".as_ptr() as *const c_char;
        assert_eq!(atoi(s), -456);
    }
});

test!("test_atoi_positive_sign" {
    // 测试 atoi 带加号的正数。
    unsafe {
        let s = b"+789".as_ptr() as *const c_char;
        assert_eq!(atoi(s), 789);
    }
});

test!("test_atoi_zero" {
    // 测试 atoi 零。
    unsafe {
        let s = b"0".as_ptr() as *const c_char;
        assert_eq!(atoi(s), 0);
    }
});

test!("test_atoi_leading_whitespace" {
    // 测试 atoi 前导空白。
    unsafe {
        let s = b"   \t\n42".as_ptr() as *const c_char;
        assert_eq!(atoi(s), 42);
    }
});

test!("test_atoi_empty" {
    // 测试 atoi 空字符串返回 0。
    unsafe {
        let s = b"".as_ptr() as *const c_char;
        assert_eq!(atoi(s), 0);
    }
});

test!("test_atoi_no_digits" {
    // 测试 atoi 无有效数字返回 0。
    unsafe {
        let s = b"abc".as_ptr() as *const c_char;
        assert_eq!(atoi(s), 0);
    }
});

test!("test_atoi_max" {
    // 测试 atoi 最大合法值。
    unsafe {
        let s = b"2147483647".as_ptr() as *const c_char;
        assert_eq!(atoi(s), i32::MAX);
    }
});

test!("test_atoi_min" {
    // 测试 atoi 最小合法值。
    unsafe {
        let s = b"-2147483648".as_ptr() as *const c_char;
        assert_eq!(atoi(s), i32::MIN);
    }
});

// ---- atol 测试 ----

test!("test_atol_positive" {
    // 测试 atol 基本正数解析。
    unsafe {
        let s = b"123".as_ptr() as *const c_char;
        assert_eq!(atol(s), 123);
    }
});

test!("test_atol_negative" {
    // 测试 atol 负数解析。
    unsafe {
        let s = b"-456".as_ptr() as *const c_char;
        assert_eq!(atol(s), -456);
    }
});

test!("test_atol_zero" {
    // 测试 atol 零。
    unsafe {
        let s = b"0".as_ptr() as *const c_char;
        assert_eq!(atol(s), 0);
    }
});

test!("test_atol_i64_max" {
    // 测试 atol i64::MAX。
    unsafe {
        let s = b"9223372036854775807".as_ptr() as *const c_char;
        assert_eq!(atol(s), i64::MAX);
    }
});

test!("test_atol_i64_min" {
    // 测试 atol i64::MIN。
    unsafe {
        let s = b"-9223372036854775808".as_ptr() as *const c_char;
        assert_eq!(atol(s), i64::MIN);
    }
});

test!("test_atol_empty" {
    // 测试 atol 空字符串。
    unsafe {
        let s = b"".as_ptr() as *const c_char;
        assert_eq!(atol(s), 0);
    }
});

// ---- atoll 测试 ----

test!("test_atoll_basic" {
    // 测试 atoll 基本解析（与 atol 行为相同，均返回 i64）。
    unsafe {
        let s = b"9876543210".as_ptr() as *const c_char;
        assert_eq!(atoll(s), 9876543210);
    }
});

test!("test_atoll_negative" {
    // 测试 atoll 负数。
    unsafe {
        let s = b"-9876543210".as_ptr() as *const c_char;
        assert_eq!(atoll(s), -9876543210);
    }
});

test!("test_atoll_zero" {
    // 测试 atoll 零。
    unsafe {
        let s = b"0".as_ptr() as *const c_char;
        assert_eq!(atoll(s), 0);
    }
});

test!("test_atoll_leading_whitespace" {
    // 测试 atoll 带前导空白。
    unsafe {
        let s = b"  \t  -42".as_ptr() as *const c_char;
        assert_eq!(atoll(s), -42);
    }
});

test!("test_atoll_empty" {
    // 测试 atoll 空字符串。
    unsafe {
        let s = b"".as_ptr() as *const c_char;
        assert_eq!(atoll(s), 0);
    }
});
