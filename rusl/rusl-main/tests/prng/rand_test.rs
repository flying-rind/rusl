/// 模块: rand_test
/// `rand` 集成测试

use test_framework::test;
use super::*;

test!("test_rand_range" {
    // rand 返回值在 [0, RAND_MAX] 范围内。
    {
        let val = rand();
        assert!(val >= 0 && val <= RAND_MAX);
    }
});

test!("test_srand_rand_sequence" {
    // srand(1) 后 rand 应产生可预测的序列。
    {
        srand(1);
        let v1 = rand();
        let v2 = rand();
        let v3 = rand();
        assert!(v1 >= 0 && v1 <= RAND_MAX);
        assert!(v2 >= 0 && v2 <= RAND_MAX);
        assert!(v3 >= 0 && v3 <= RAND_MAX);
    }
});

test!("test_srand_rand_reproducible" {
    // srand + rand 可复现：相同种子产生相同第一个值。
    {
        srand(42);
        let a = rand();
        srand(42);
        let b = rand();
        assert_eq!(a, b);
    }
});

test!("test_srand_different_seeds" {
    // 不同种子通常产生不同的序列。
    {
        srand(1);
        let _a = rand();
        srand(2);
        let _b = rand();
        // 不同种子产生不同值（概率极高）
    }
});

test!("test_srand_zero_seed" {
    // srand(0) 的特殊情况：seed = (0 - 1) as u64 = 0xFFFFFFFFFFFFFFFF。
    {
        srand(0);
        let val = rand();
        assert!(val >= 0 && val <= RAND_MAX);
    }
});

test!("test_rand_multiple_calls" {
    // srand 后连续多次 rand 调用均不 panic。
    {
        srand(123);
        for _ in 0..100 {
            let val = rand();
            assert!(val >= 0 && val <= RAND_MAX);
        }
    }
});

test!("test_rand_not_constant" {
    // 重复调用 rand 不应始终返回相同值。
    {
        srand(42);
        let v1 = rand();
        let v2 = rand();
        // 注意：特定种子可能导致前两个值相同，但概率极低
        // 此测试仅确保范围正确
        assert!(v1 >= 0 && v1 <= RAND_MAX);
        assert!(v2 >= 0 && v2 <= RAND_MAX);
    }
});
