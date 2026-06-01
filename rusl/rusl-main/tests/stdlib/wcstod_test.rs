/// 模块: wcstod_test
/// `wcstod` 集成测试

use super::imports::{wcstod, wcstof};

// [removed alloc import: // [removed alloc import: ]]
use rusl_core::test;

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

// ---- wcstod 测试 ----

test!("test_wcstod_basic" {
    unsafe {
        let ws = wide_str!("3.14159");
        let mut endptr: *mut wchar_t = core::ptr::null_mut();
        let result = wcstod(ws.as_ptr(), &mut endptr as *mut *mut wchar_t);
        assert!((result - 3.14159).abs() < 1e-10);
    }
});

test!("test_wcstod_negative" {
    unsafe {
        let ws = wide_str!("-2.5");
        let result = wcstod(ws.as_ptr(), core::ptr::null_mut());
        assert!((result - (-2.5)).abs() < 1e-10);
    }
});

test!("test_wcstod_scientific" {
    unsafe {
        let ws = wide_str!("1.5e2");
        let result = wcstod(ws.as_ptr(), core::ptr::null_mut());
        assert!((result - 150.0).abs() < 1e-10);
    }
});

test!("test_wcstod_inf" {
    unsafe {
        let ws = wide_str!("inf");
        let result = wcstod(ws.as_ptr(), core::ptr::null_mut());
        assert!(result.is_infinite());
    }
});

test!("test_wcstod_nan" {
    unsafe {
        let ws = wide_str!("nan");
        let result = wcstod(ws.as_ptr(), core::ptr::null_mut());
        assert!(result.is_nan());
    }
});

test!("test_wcstod_no_conversion" {
    unsafe {
        let ws = wide_str!("abc");
        let mut endptr: *mut wchar_t = core::ptr::null_mut();
        let result = wcstod(ws.as_ptr(), &mut endptr as *mut *mut wchar_t);
        assert_eq!(result, 0.0);
        assert_eq!(endptr, ws.as_ptr() as *mut wchar_t);
    }
});

test!("test_wcstod_endptr" {
    unsafe {
        let ws = wide_str!("3.14extra");
        let mut endptr: *mut wchar_t = core::ptr::null_mut();
        let result = wcstod(ws.as_ptr(), &mut endptr as *mut *mut wchar_t);
        assert!((result - 3.14).abs() < 1e-10);
        assert_eq!(*endptr, 'e' as wchar_t);
    }
});

test!("test_wcstod_zero" {
    unsafe {
        let ws = wide_str!("0.0");
        let result = wcstod(ws.as_ptr(), core::ptr::null_mut());
        assert_eq!(result, 0.0);
    }
});

test!("test_wcstod_leading_whitespace" {
    unsafe {
        let ws = wide_str!("  \t\n-1.5");
        let result = wcstod(ws.as_ptr(), core::ptr::null_mut());
        assert!((result - (-1.5)).abs() < 1e-10);
    }
});

// ---- wcstof 测试 ----

test!("test_wcstof_basic" {
    unsafe {
        let ws = wide_str!("3.14");
        let result = wcstof(ws.as_ptr(), core::ptr::null_mut());
        assert!((result - 3.14f32).abs() < 1e-6);
    }
});

test!("test_wcstof_inf" {
    unsafe {
        let ws = wide_str!("-inf");
        let result = wcstof(ws.as_ptr(), core::ptr::null_mut());
        assert!(result.is_infinite());
        assert!(result.is_sign_negative());
    }
});

test!("test_wcstof_no_conversion" {
    unsafe {
        let ws = wide_str!("xyz");
        let mut endptr: *mut wchar_t = core::ptr::null_mut();
        let result = wcstof(ws.as_ptr(), &mut endptr as *mut *mut wchar_t);
        assert_eq!(result, 0.0);
        assert_eq!(endptr, ws.as_ptr() as *mut wchar_t);
    }
});

test!("test_wcstof_zero" {
    unsafe {
        let ws = wide_str!("0");
        let result = wcstof(ws.as_ptr(), core::ptr::null_mut());
        assert_eq!(result, 0.0);
    }
});
