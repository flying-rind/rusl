/// 模块: abs_test
/// `abs` 集成测试

use rusl_core::test;
use super::*;

test!("test_positive" {
    // 测试正数绝对值。
    unsafe {
        assert_eq!(abs(42), 42);
        assert_eq!(abs(1), 1);
        assert_eq!(abs(i32::MAX), i32::MAX);
    }
});

test!("test_zero" {
    // 测试零的绝对值。
    unsafe {
        assert_eq!(abs(0), 0);
    }
});

test!("test_negative" {
    // 测试负数绝对值。
    unsafe {
        assert_eq!(abs(-42), 42);
        assert_eq!(abs(-1), 1);
    }
});

test!("test_min_undefined_behavior" {
    // 测试 i32::MIN 的特殊情况（行为未定义，仅标记）。
    // 注意：i32::MIN 的绝对值无法用 i32 表示，spec 注明此时行为未定义。
    unsafe {
        // abs(i32::MIN) 导致 UB
        let _ = abs(i32::MIN);
    }
});
