/// 模块: wcstol_test
/// `wcstol` 集成测试

use super::imports::{wcstol, wcstoll, wcstoul, wcstoull, wcstoimax, wcstoumax};

// [removed alloc import: // [removed alloc import: ]]
use test_framework::test;

#[allow(non_camel_case_types)]
type wchar_t = i32;
// [removed alloc import: // [removed alloc import:]]
// [removed alloc import: // [removed alloc import:]]
// [removed alloc import: use alloc::vec::Vec;]
macro_rules! wide_str {
    ($s:expr) => {{
        let bytes = $s.as_bytes();
        let mut buf = [0i32; 128];
        let mut i = 0;
        while i < bytes.len() {
            buf[i] = bytes[i] as i32;
            i += 1;
        }
        buf[i] = 0;
        buf
    }};
}

// ---- wcstol 测试 ----

test!("test_wcstol_decimal" {
    unsafe {
        let ws = wide_str!("12345");
        let mut endptr: *mut wchar_t = core::ptr::null_mut();
        let result = wcstol(ws.as_ptr(), &mut endptr as *mut *mut wchar_t, 10);
        assert_eq!(result, 12345);
        assert_eq!(*endptr, 0);
    }
});

test!("test_wcstol_negative" {
    unsafe {
        let ws = wide_str!("-6789");
        let result = wcstol(ws.as_ptr(), core::ptr::null_mut(), 10);
        assert_eq!(result, -6789);
    }
});

test!("test_wcstol_hex" {
    unsafe {
        let ws = wide_str!("ff");
        let result = wcstol(ws.as_ptr(), core::ptr::null_mut(), 16);
        assert_eq!(result, 255);
    }
});

test!("test_wcstol_auto_hex" {
    unsafe {
        let ws = wide_str!("0xff");
        let result = wcstol(ws.as_ptr(), core::ptr::null_mut(), 0);
        assert_eq!(result, 255);
    }
});

test!("test_wcstol_auto_octal" {
    unsafe {
        let ws = wide_str!("077");
        let result = wcstol(ws.as_ptr(), core::ptr::null_mut(), 0);
        assert_eq!(result, 63);
    }
});

test!("test_wcstol_auto_decimal" {
    unsafe {
        let ws = wide_str!("123");
        let result = wcstol(ws.as_ptr(), core::ptr::null_mut(), 0);
        assert_eq!(result, 123);
    }
});

test!("test_wcstol_leading_whitespace" {
    unsafe {
        let ws = wide_str!("  \t\n42");
        let result = wcstol(ws.as_ptr(), core::ptr::null_mut(), 10);
        assert_eq!(result, 42);
    }
});

test!("test_wcstol_no_digits" {
    unsafe {
        let ws = wide_str!("abc");
        let mut endptr: *mut wchar_t = core::ptr::null_mut();
        let result = wcstol(ws.as_ptr(), &mut endptr as *mut *mut wchar_t, 10);
        assert_eq!(result, 0);
        assert_eq!(endptr, ws.as_ptr() as *mut wchar_t);
    }
});

test!("test_wcstol_endptr" {
    unsafe {
        let ws = wide_str!("123abc");
        let mut endptr: *mut wchar_t = core::ptr::null_mut();
        let result = wcstol(ws.as_ptr(), &mut endptr as *mut *mut wchar_t, 10);
        assert_eq!(result, 123);
        assert_eq!(*endptr, 'a' as wchar_t);
    }
});

test!("test_wcstol_overflow_positive" {
    unsafe {
        let ws = wide_str!("99999999999999999999999999999");
        let result = wcstol(ws.as_ptr(), core::ptr::null_mut(), 10);
        assert_eq!(result, i64::MAX);
    }
});

test!("test_wcstol_overflow_negative" {
    unsafe {
        let ws = wide_str!("-99999999999999999999999999999");
        let result = wcstol(ws.as_ptr(), core::ptr::null_mut(), 10);
        assert_eq!(result, i64::MIN);
    }
});

test!("test_wcstol_max_value" {
    unsafe {
        let ws = wide_str!("9223372036854775807");
        let result = wcstol(ws.as_ptr(), core::ptr::null_mut(), 10);
        assert_eq!(result, i64::MAX);
    }
});

test!("test_wcstol_min_value" {
    unsafe {
        let ws = wide_str!("-9223372036854775808");
        let result = wcstol(ws.as_ptr(), core::ptr::null_mut(), 10);
        assert_eq!(result, i64::MIN);
    }
});

test!("test_wcstol_zero" {
    unsafe {
        let ws = wide_str!("0");
        let result = wcstol(ws.as_ptr(), core::ptr::null_mut(), 10);
        assert_eq!(result, 0);
    }
});

// ---- wcstoll 测试 ----

test!("test_wcstoll_basic" {
    unsafe {
        let ws = wide_str!("9876543210");
        let result = wcstoll(ws.as_ptr(), core::ptr::null_mut(), 10);
        assert_eq!(result, 9876543210);
    }
});

// ---- wcstoul 测试 ----

test!("test_wcstoul_basic" {
    unsafe {
        let ws = wide_str!("12345");
        let result = wcstoul(ws.as_ptr(), core::ptr::null_mut(), 10);
        assert_eq!(result, 12345);
    }
});

test!("test_wcstoul_u64_max" {
    unsafe {
        let ws = wide_str!("18446744073709551615");
        let result = wcstoul(ws.as_ptr(), core::ptr::null_mut(), 10);
        assert_eq!(result, u64::MAX);
    }
});

test!("test_wcstoul_overflow" {
    unsafe {
        let ws = wide_str!("99999999999999999999999999999");
        let result = wcstoul(ws.as_ptr(), core::ptr::null_mut(), 10);
        assert_eq!(result, u64::MAX);
    }
});

// ---- wcstoull 测试 ----

test!("test_wcstoull_basic" {
    unsafe {
        let ws = wide_str!("255");
        let result = wcstoull(ws.as_ptr(), core::ptr::null_mut(), 10);
        assert_eq!(result, 255);
    }
});

// ---- wcstoimax 测试 ----

test!("test_wcstoimax_basic" {
    unsafe {
        let ws = wide_str!("-123");
        let result = wcstoimax(ws.as_ptr(), core::ptr::null_mut(), 10);
        assert_eq!(result, -123);
    }
});

test!("test_wcstoimax_auto_hex" {
    unsafe {
        let ws = wide_str!("0x1A2B");
        let result = wcstoimax(ws.as_ptr(), core::ptr::null_mut(), 0);
        assert_eq!(result, 0x1A2B);
    }
});

test!("test_wcstoimax_no_digits" {
    unsafe {
        let ws = wide_str!("xyz");
        let mut endptr: *mut wchar_t = core::ptr::null_mut();
        let result = wcstoimax(ws.as_ptr(), &mut endptr as *mut *mut wchar_t, 10);
        assert_eq!(result, 0);
        assert_eq!(endptr, ws.as_ptr() as *mut wchar_t);
    }
});

// ---- wcstoumax 测试 ----

test!("test_wcstoumax_basic" {
    unsafe {
        let ws = wide_str!("255");
        let result = wcstoumax(ws.as_ptr(), core::ptr::null_mut(), 10);
        assert_eq!(result, 255);
    }
});

test!("test_wcstoumax_hex" {
    unsafe {
        let ws = wide_str!("ff");
        let result = wcstoumax(ws.as_ptr(), core::ptr::null_mut(), 16);
        assert_eq!(result, 255);
    }
});
