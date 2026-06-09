/// 模块: strtol_test
/// `strtol` 集成测试

#[cfg(feature = "rusl")]
extern crate alloc;

use core::ffi::{c_char};
use super::imports::{strtol, strtoll, strtoul, strtoull, strtoimax, strtoumax};
// [removed alloc import: use alloc::ffi::CString;]
use test_framework::test;

// ---- strtol 测试 ----

test!("test_strtol_decimal" {
    unsafe {
        let s = b"12345\0".as_ptr() as *const c_char;
        let mut endptr: *mut c_char = core::ptr::null_mut();
        let result = strtol(s, &mut endptr as *mut *mut c_char, 10);
        assert_eq!(result, 12345);
        assert_eq!(*endptr, 0);
    }
});

test!("test_strtol_negative" {
    unsafe {
        let s = b"-6789\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 10);
        assert_eq!(result, -6789);
    }
});

test!("test_strtol_hex" {
    unsafe {
        let s = b"ff\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 16);
        assert_eq!(result, 255);
    }
});

test!("test_strtol_octal" {
    unsafe {
        let s = b"77\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 8);
        assert_eq!(result, 63);
    }
});

test!("test_strtol_binary" {
    unsafe {
        let s = b"1010\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 2);
        assert_eq!(result, 10);
    }
});

test!("test_strtol_auto_hex" {
    unsafe {
        let s = b"0xff\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 0);
        assert_eq!(result, 255);
    }
});

test!("test_strtol_auto_octal" {
    unsafe {
        let s = b"077\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 0);
        assert_eq!(result, 63);
    }
});

test!("test_strtol_auto_decimal" {
    unsafe {
        let s = b"123\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 0);
        assert_eq!(result, 123);
    }
});

test!("test_strtol_leading_whitespace" {
    unsafe {
        let s = b"  \t\n42\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 10);
        assert_eq!(result, 42);
    }
});

test!("test_strtol_no_digits" {
    unsafe {
        let s = b"abc\0".as_ptr() as *const c_char;
        let mut endptr: *mut c_char = core::ptr::null_mut();
        let result = strtol(s, &mut endptr as *mut *mut c_char, 10);
        assert_eq!(result, 0);
        assert_eq!(endptr, s as *mut c_char);
    }
});

test!("test_strtol_endptr" {
    unsafe {
        let s = b"123abc\0".as_ptr() as *const c_char;
        let mut endptr: *mut c_char = core::ptr::null_mut();
        let result = strtol(s, &mut endptr as *mut *mut c_char, 10);
        assert_eq!(result, 123);
        assert_eq!(*endptr as u8, b'a');
    }
});

test!("test_strtol_overflow_positive" {
    unsafe {
        let s = b"99999999999999999999999999999\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 10);
        assert_eq!(result, i64::MAX);
    }
});

test!("test_strtol_overflow_negative" {
    unsafe {
        let s = b"-99999999999999999999999999999\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 10);
        assert_eq!(result, i64::MIN);
    }
});

test!("test_strtol_max_value" {
    unsafe {
        let s = b"9223372036854775807\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 10);
        assert_eq!(result, i64::MAX);
    }
});

test!("test_strtol_min_value" {
    unsafe {
        let s = b"-9223372036854775808\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 10);
        assert_eq!(result, i64::MIN);
    }
});

test!("test_strtol_zero" {
    unsafe {
        let s = b"0\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 10);
        assert_eq!(result, 0);
    }
});

test!("test_strtol_positive_sign" {
    unsafe {
        let s = b"+42\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 10);
        assert_eq!(result, 42);
    }
});

test!("test_strtol_base36" {
    unsafe {
        let s = b"zz\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 36);
        assert_eq!(result, 35 * 36 + 35);
    }
});

test!("test_strtol_null_endptr" {
    unsafe {
        let s = b"123\0".as_ptr() as *const c_char;
        let result = strtol(s, core::ptr::null_mut(), 10);
        assert_eq!(result, 123);
    }
});

// ---- strtoll 测试 ----

test!("test_strtoll_basic" {
    unsafe {
        let s = b"9876543210\0".as_ptr() as *const c_char;
        let result = strtoll(s, core::ptr::null_mut(), 10);
        assert_eq!(result, 9876543210);
    }
});

test!("test_strtoll_hex" {
    unsafe {
        let s = b"0xabcdef\0".as_ptr() as *const c_char;
        let result = strtoll(s, core::ptr::null_mut(), 16);
        assert_eq!(result, 0xabcdef);
    }
});

// ---- strtoul 测试 ----

test!("test_strtoul_basic" {
    unsafe {
        let s = b"12345\0".as_ptr() as *const c_char;
        let result = strtoul(s, core::ptr::null_mut(), 10);
        assert_eq!(result, 12345);
    }
});

test!("test_strtoul_negative" {
    unsafe {
        let s = b"-1\0".as_ptr() as *const c_char;
        let mut endptr: *mut c_char = core::ptr::null_mut();
        let result = strtoul(s, &mut endptr as *mut *mut c_char, 10);
        // strtoul("-1") 取反后应返回 u64::MAX（非溢出）
        assert_eq!(result, u64::MAX);
    }
});

test!("test_strtoul_u64_max" {
    unsafe {
        let s = b"18446744073709551615\0".as_ptr() as *const c_char;
        let result = strtoul(s, core::ptr::null_mut(), 10);
        assert_eq!(result, u64::MAX);
    }
});

test!("test_strtoul_overflow" {
    unsafe {
        let s = b"99999999999999999999999999999\0".as_ptr() as *const c_char;
        let result = strtoul(s, core::ptr::null_mut(), 10);
        assert_eq!(result, u64::MAX);
    }
});

// ---- strtoull 测试 ----

test!("test_strtoull_basic" {
    unsafe {
        let s = b"42\0".as_ptr() as *const c_char;
        let result = strtoull(s, core::ptr::null_mut(), 10);
        assert_eq!(result, 42);
    }
});

// ---- strtoimax 测试 ----

test!("test_strtoimax_basic" {
    unsafe {
        let s = b"-123\0".as_ptr() as *const c_char;
        let result = strtoimax(s, core::ptr::null_mut(), 10);
        assert_eq!(result, -123);
    }
});

test!("test_strtoimax_auto_hex" {
    unsafe {
        let s = b"0x1A2B\0".as_ptr() as *const c_char;
        let result = strtoimax(s, core::ptr::null_mut(), 0);
        assert_eq!(result, 0x1A2B);
    }
});

test!("test_strtoimax_no_digits" {
    unsafe {
        let s = b"xyz\0".as_ptr() as *const c_char;
        let mut endptr: *mut c_char = core::ptr::null_mut();
        let result = strtoimax(s, &mut endptr as *mut *mut c_char, 10);
        assert_eq!(result, 0);
        assert_eq!(endptr, s as *mut c_char);
    }
});

// ---- strtoumax 测试 ----

test!("test_strtoumax_basic" {
    unsafe {
        let s = b"255\0".as_ptr() as *const c_char;
        let result = strtoumax(s, core::ptr::null_mut(), 10);
        assert_eq!(result, 255);
    }
});

test!("test_strtoumax_hex" {
    unsafe {
        let s = b"ff\0".as_ptr() as *const c_char;
        let result = strtoumax(s, core::ptr::null_mut(), 16);
        assert_eq!(result, 255);
    }
});
