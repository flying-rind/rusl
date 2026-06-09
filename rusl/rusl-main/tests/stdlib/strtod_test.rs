/// 模块: strtod_test
/// `strtod` 集成测试

use core::ffi::{c_char};
use super::imports::{strtod, strtof};

// [removed alloc import: use alloc::ffi::CString;]
use test_framework::test;

// ---- strtod 测试 ----

test!("test_strtod_basic" {
    unsafe {
        let s = b"3.14159\0".as_ptr() as *const c_char;
        let mut endptr: *mut c_char = core::ptr::null_mut();
        let result = strtod(s, &mut endptr as *mut *mut c_char);
        assert!((result - 3.14159).abs() < 1e-10);
    }
});

test!("test_strtod_negative" {
    unsafe {
        let s = b"-2.5\0".as_ptr() as *const c_char;
        let mut endptr: *mut c_char = core::ptr::null_mut();
        let result = strtod(s, &mut endptr as *mut *mut c_char);
        assert!((result - (-2.5)).abs() < 1e-10);
    }
});

test!("test_strtod_scientific" {
    unsafe {
        let s = b"1.5e2\0".as_ptr() as *const c_char;
        let result = strtod(s, core::ptr::null_mut());
        assert!((result - 150.0).abs() < 1e-10);
    }
});

test!("test_strtod_hex" {
    unsafe {
        let s = b"0x1.ffffp+10\0".as_ptr() as *const c_char;
        let result = strtod(s, core::ptr::null_mut());
        // 0x1.ffffp+10 = 2047.984375
        assert!((result - 2047.984375).abs() < 1e-10);
    }
});

test!("test_strtod_inf" {
    unsafe {
        let s = b"inf\0".as_ptr() as *const c_char;
        let result = strtod(s, core::ptr::null_mut());
        assert!(result.is_infinite());
        assert!(result.is_sign_positive());
    }
});

test!("test_strtod_neg_inf" {
    unsafe {
        let s = b"-infinity\0".as_ptr() as *const c_char;
        let result = strtod(s, core::ptr::null_mut());
        assert!(result.is_infinite());
        assert!(result.is_sign_negative());
    }
});

test!("test_strtod_nan" {
    unsafe {
        let s = b"nan\0".as_ptr() as *const c_char;
        let result = strtod(s, core::ptr::null_mut());
        assert!(result.is_nan());
    }
});

test!("test_strtod_no_conversion" {
    unsafe {
        let s = b"abc\0".as_ptr() as *const c_char;
        let mut endptr: *mut c_char = core::ptr::null_mut();
        let result = strtod(s, &mut endptr as *mut *mut c_char);
        assert_eq!(result, 0.0);
        assert_eq!(endptr, s as *mut c_char);
    }
});

test!("test_strtod_endptr" {
    unsafe {
        let s = b"3.14extra\0".as_ptr() as *const c_char;
        let mut endptr: *mut c_char = core::ptr::null_mut();
        let result = strtod(s, &mut endptr as *mut *mut c_char);
        assert!((result - 3.14).abs() < 1e-10);
        assert_eq!(*endptr as u8, b'e');
    }
});

test!("test_strtod_zero" {
    unsafe {
        let s = b"0.0\0".as_ptr() as *const c_char;
        let result = strtod(s, core::ptr::null_mut());
        assert_eq!(result, 0.0);
    }
});

test!("test_strtod_leading_whitespace" {
    unsafe {
        let s = b"   \t\n-1.5\0".as_ptr() as *const c_char;
        let result = strtod(s, core::ptr::null_mut());
        assert!((result - (-1.5)).abs() < 1e-10);
    }
});

test!("test_strtod_null_endptr" {
    unsafe {
        let s = b"123.456\0".as_ptr() as *const c_char;
        let result = strtod(s, core::ptr::null_mut());
        assert!((result - 123.456).abs() < 1e-10);
    }
});

test!("test_strtod_overflow" {
    unsafe {
        let s = b"1e1000\0".as_ptr() as *const c_char;
        let result = strtod(s, core::ptr::null_mut());
        assert!(result.is_infinite());
    }
});

// ---- strtof 测试 ----

test!("test_strtof_basic" {
    unsafe {
        let s = b"3.14\0".as_ptr() as *const c_char;
        let result = strtof(s, core::ptr::null_mut());
        assert!((result - 3.14f32).abs() < 1e-6);
    }
});

test!("test_strtof_inf" {
    unsafe {
        let s = b"inf\0".as_ptr() as *const c_char;
        let result = strtof(s, core::ptr::null_mut());
        assert!(result.is_infinite());
    }
});

test!("test_strtof_no_conversion" {
    unsafe {
        let s = b"xyz\0".as_ptr() as *const c_char;
        let mut endptr: *mut c_char = core::ptr::null_mut();
        let result = strtof(s, &mut endptr as *mut *mut c_char);
        assert_eq!(result, 0.0);
        assert_eq!(endptr, s as *mut c_char);
    }
});

test!("test_strtof_zero" {
    unsafe {
        let s = b"0\0".as_ptr() as *const c_char;
        let result = strtof(s, core::ptr::null_mut());
        assert_eq!(result, 0.0);
    }
});
