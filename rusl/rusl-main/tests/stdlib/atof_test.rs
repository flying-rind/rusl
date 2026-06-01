/// 模块: atof_test
/// `atof` 集成测试

use core::ffi::{c_char};
use super::imports::{atof};

// [removed alloc import: // [removed alloc import: use alloc::ffi::CString;]]
use rusl_core::test;

test!("test_basic_decimal" {
    // 测试基本的十进制字符串解析。
    unsafe {
        let s = b"3.14".as_ptr() as *const c_char;
        let result = atof(s);
        assert!((result - 3.14).abs() < 1e-10);
    }
});

test!("test_negative" {
    // 测试负数的解析。
    unsafe {
        let s = b"-2.5".as_ptr() as *const c_char;
        let result = atof(s);
        assert!((result - (-2.5)).abs() < 1e-10);
    }
});

test!("test_scientific_notation" {
    // 测试科学计数法。
    unsafe {
        let s = b"1.5e2".as_ptr() as *const c_char;
        let result = atof(s);
        assert!((result - 150.0).abs() < 1e-10);
    }
});

test!("test_zero" {
    // 测试零值。
    unsafe {
        let s = b"0.0".as_ptr() as *const c_char;
        let result = atof(s);
        assert_eq!(result, 0.0);
    }
});

test!("test_no_valid_digits" {
    // 测试无有效数字的字符串。
    unsafe {
        let s = b"abc".as_ptr() as *const c_char;
        let result = atof(s);
        assert_eq!(result, 0.0);
    }
});

test!("test_empty_string" {
    // 测试空字符串（只有 null 终止符）。
    unsafe {
        let s = b"".as_ptr() as *const c_char;
        let result = atof(s);
        assert_eq!(result, 0.0);
    }
});

test!("test_whitespace_then_no_digits" {
    // 测试字符串仅含空白字符后跟非数字。
    unsafe {
        let s = b"   abc".as_ptr() as *const c_char;
        let result = atof(s);
        assert_eq!(result, 0.0);
    }
});
