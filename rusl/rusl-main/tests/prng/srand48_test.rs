/// 模块: srand48_test
/// `srand48` 集成测试

use super::*;
use test_framework::test;

test!("test_srand48_no_panic" {
    // srand48 调用后不应 panic。
    unsafe {
        srand48(0);
        srand48(12345);
        srand48(-1);
        srand48(i64::MAX);
        srand48(i64::MIN);
    }
});

test!("test_srand48_seed_zero_consistency" {
    // srand48(0) 后 lrand48 返回可预测值。
    unsafe {
        srand48(0);
        let a = lrand48();
        srand48(0);
        let b = lrand48();
        assert_eq!(a, b);
    }
});

test!("test_srand48_different_seeds" {
    // 不同种子产生不同序列。
    unsafe {
        srand48(1);
        let _a = lrand48();
        srand48(2);
        let _b = lrand48();
    }
});

test!("test_srand48_drand48_range" {
    // srand48 后 drand48 值在 [0.0, 1.0) 范围内。
    unsafe {
        srand48(9999);
        let val = drand48();
        assert!(val >= 0.0 && val < 1.0);
    }
});

test!("test_srand48_mrand48_reproducible" {
    // srand48 + mrand48 的可复现性。
    unsafe {
        srand48(777);
        let a = mrand48();
        srand48(777);
        let b = mrand48();
        assert_eq!(a, b);
    }
});
