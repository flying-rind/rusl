/// 模块: labs_test
/// `labs` 集成测试

// use core::ffi::{c_void};
use super::imports::{labs, llabs, imaxabs};
use rusl_core::test;

// ---- labs 测试 ----

test!("test_labs_positive" {
    // 测试 labs 正数。
    unsafe {
        assert_eq!(labs(42), 42);
        assert_eq!(labs(1), 1);
        assert_eq!(labs(i64::MAX), i64::MAX);
    }
});

test!("test_labs_zero" {
    // 测试 labs 零。
    unsafe {
        assert_eq!(labs(0), 0);
    }
});

test!("test_labs_negative" {
    // 测试 labs 负数。
    unsafe {
        assert_eq!(labs(-42), 42);
        assert_eq!(labs(-1), 1);
    }
});

test!("test_labs_large_negative" {
    // 测试 labs 大负数。
    unsafe {
        assert_eq!(labs(-9223372036854775807), 9223372036854775807);
    }
});

// ---- llabs 测试 ----

test!("test_llabs_positive" {
    // 测试 llabs 正数。
    unsafe {
        assert_eq!(llabs(123456789), 123456789);
    }
});

test!("test_llabs_zero" {
    // 测试 llabs 零。
    unsafe {
        assert_eq!(llabs(0), 0);
    }
});

test!("test_llabs_negative" {
    // 测试 llabs 负数。
    unsafe {
        assert_eq!(llabs(-987654321), 987654321);
    }
});

// ---- imaxabs 测试 ----

test!("test_imaxabs_positive" {
    // 测试 imaxabs 正数。
    unsafe {
        assert_eq!(imaxabs(100), 100);
    }
});

test!("test_imaxabs_zero" {
    // 测试 imaxabs 零。
    unsafe {
        assert_eq!(imaxabs(0), 0);
    }
});

test!("test_imaxabs_negative" {
    // 测试 imaxabs 负数。
    unsafe {
        assert_eq!(imaxabs(-100), 100);
    }
});
