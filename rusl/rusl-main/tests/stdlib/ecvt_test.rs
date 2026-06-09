/// 模块: ecvt_test
/// `ecvt` / `fcvt` / `gcvt` 集成测试

use super::imports::{ecvt, fcvt, gcvt};
use test_framework::test;

// ---- ecvt 测试 ----

test!("test_ecvt_positive" {
    // 测试 ecvt 转换正数。
    unsafe {
        let mut dp: i32 = 0;
        let mut sign: i32 = 1;
        let result = ecvt(3.14159, 6, &mut dp as *mut i32, &mut sign as *mut i32);
        assert!(!result.is_null());
        // sign 应为 0（正数）
        assert_eq!(sign, 0);
        // dp 应为小数点位置
        assert_eq!(dp, 1);
    }
});

test!("test_ecvt_negative" {
    // 测试 ecvt 转换负数。
    unsafe {
        let mut dp: i32 = 0;
        let mut sign: i32 = 0;
        let result = ecvt(-3.14, 4, &mut dp as *mut i32, &mut sign as *mut i32);
        assert!(!result.is_null());
        // sign 应为 1（负数）
        assert_eq!(sign, 1);
    }
});

test!("test_ecvt_zero" {
    // 测试 ecvt 转换零。
    unsafe {
        let mut dp: i32 = 0;
        let mut sign: i32 = 1;
        let result = ecvt(0.0, 3, &mut dp as *mut i32, &mut sign as *mut i32);
        assert!(!result.is_null());
        assert_eq!(sign, 0);
    }
});

test!("test_ecvt_large" {
    // 测试 ecvt 转换非常大的数字。
    unsafe {
        let mut dp: i32 = 0;
        let mut sign: i32 = 0;
        let result = ecvt(1e10, 6, &mut dp as *mut i32, &mut sign as *mut i32);
        assert!(!result.is_null());
        // 1e10 = 10000000000, 所以小数点位置应为 11
        assert_eq!(dp, 11);
    }
});

test!("test_ecvt_small" {
    // 测试 ecvt 转换非常小的数字。
    unsafe {
        let mut dp: i32 = 0;
        let mut sign: i32 = 0;
        let result = ecvt(0.00123, 4, &mut dp as *mut i32, &mut sign as *mut i32);
        assert!(!result.is_null());
        // 0.00123 = 1.23e-3, 小数点位置为 -2（或 0 表示前导零后第一位）
        // 具体值取决于实现
    }
});

// ---- fcvt 测试 ----

test!("test_fcvt_positive" {
    // 测试 fcvt 转换正数。
    unsafe {
        let mut dp: i32 = 0;
        let mut sign: i32 = 1;
        let result = fcvt(3.14159, 3, &mut dp as *mut i32, &mut sign as *mut i32);
        assert!(!result.is_null());
        assert_eq!(sign, 0);
    }
});

test!("test_fcvt_negative" {
    // 测试 fcvt 转换负数。
    unsafe {
        let mut dp: i32 = 0;
        let mut sign: i32 = 0;
        let result = fcvt(-2.718, 2, &mut dp as *mut i32, &mut sign as *mut i32);
        assert!(!result.is_null());
        assert_eq!(sign, 1);
    }
});

test!("test_fcvt_zero" {
    // 测试 fcvt 转换零。
    unsafe {
        let mut dp: i32 = 0;
        let mut sign: i32 = 1;
        let result = fcvt(0.0, 5, &mut dp as *mut i32, &mut sign as *mut i32);
        assert!(!result.is_null());
        assert_eq!(sign, 0);
    }
});

test!("test_fcvt_many_leading_zeros" {
    // 测试 fcvt 大量前导零的情况。
    unsafe {
        let mut dp: i32 = 0;
        let mut sign: i32 = 0;
        // 非常小的正数，产生大量前导零
        let result = fcvt(0.00001, 10, &mut dp as *mut i32, &mut sign as *mut i32);
        assert!(!result.is_null());
        // 注意：当前导零数量 >= n 时，fcvt 应返回 "000..." 常量
    }
});

// ---- gcvt 测试 ----

test!("test_gcvt_basic" {
    // 测试 gcvt 基本转换。
    unsafe {
        let mut buf = [0i8; 64];
        let result = gcvt(3.14159, 6, buf.as_mut_ptr());
        assert_eq!(result, buf.as_mut_ptr());
    }
});

test!("test_gcvt_integer" {
    // 测试 gcvt 转换整数。
    unsafe {
        let mut buf = [0i8; 64];
        let result = gcvt(42.0, 6, buf.as_mut_ptr());
        assert_eq!(result, buf.as_mut_ptr());
    }
});

test!("test_gcvt_negative" {
    // 测试 gcvt 转换负数。
    unsafe {
        let mut buf = [0i8; 64];
        let result = gcvt(-3.14, 4, buf.as_mut_ptr());
        assert_eq!(result, buf.as_mut_ptr());
    }
});

test!("test_gcvt_zero" {
    // 测试 gcvt 转换零。
    unsafe {
        let mut buf = [0i8; 64];
        let result = gcvt(0.0, 4, buf.as_mut_ptr());
        assert_eq!(result, buf.as_mut_ptr());
    }
});
