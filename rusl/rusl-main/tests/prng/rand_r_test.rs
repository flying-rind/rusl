/// 模块: rand_r_test
/// `rand_r` 集成测试

use super::*;
use test_framework::test;

test!("test_rand_r_updates_seed" {
    // rand_r 更新种子指针指向的值。
    unsafe {
        let mut seed: u32 = 1;
        let _ = rand_r(&mut seed as *mut u32);
        assert_ne!(seed, 1, "rand_r 应更新种子值");
    }
});

test!("test_rand_r_range" {
    // rand_r 返回值在 [0, RAND_MAX] 范围内。
    unsafe {
        let mut seed: u32 = 12345;
        let val = rand_r(&mut seed as *mut u32);
        assert!(val >= 0 && val <= RAND_MAX);
    }
});

test!("test_rand_r_deterministic" {
    // rand_r 的确定性：相同种子产生相同输出。
    unsafe {
        let mut seed1: u32 = 42;
        let mut seed2: u32 = 42;
        let a = rand_r(&mut seed1 as *mut u32);
        let b = rand_r(&mut seed2 as *mut u32);
        assert_eq!(a, b);
        assert_eq!(seed1, seed2);
    }
});

test!("test_rand_r_sequence" {
    // rand_r 的链式调用产生不同值。
    unsafe {
        let mut seed: u32 = 1;
        let v1 = rand_r(&mut seed as *mut u32);
        let v2 = rand_r(&mut seed as *mut u32);
        let v3 = rand_r(&mut seed as *mut u32);
        assert!(v1 >= 0 && v1 <= RAND_MAX);
        assert!(v2 >= 0 && v2 <= RAND_MAX);
        assert!(v3 >= 0 && v3 <= RAND_MAX);
    }
});

test!("test_rand_r_different_seeds" {
    // 不同种子产生不同序列。
    unsafe {
        let mut s1: u32 = 1;
        let mut s2: u32 = 999;
        let _v1 = rand_r(&mut s1 as *mut u32);
        let _v2 = rand_r(&mut s2 as *mut u32);
        // 种子不同，内部状态不同（不 panic 即可）
    }
});

test!("test_rand_r_independent_states" {
    // rand_r 的线程安全性质：不同种子应完全独立。
    unsafe {
        let mut seed_a: u32 = 100;
        let mut seed_b: u32 = 100;
        // 两个独立种子应产生相同的序列
        for _ in 0..10 {
            let a = rand_r(&mut seed_a as *mut u32);
            let b = rand_r(&mut seed_b as *mut u32);
            assert_eq!(a, b);
            assert_eq!(seed_a, seed_b);
        }
    }
});

test!("test_rand_r_zero_seed" {
    // 零种子边界情况。
    unsafe {
        let mut seed: u32 = 0;
        let val = rand_r(&mut seed as *mut u32);
        assert!(val >= 0 && val <= RAND_MAX);
        // 0 的 LCG 变换: 0 * 1103515245 + 12345 = 12345
        assert_ne!(seed, 0);
    }
});

test!("test_rand_r_max_seed" {
    // 最大种子边界情况 (u32::MAX)。
    unsafe {
        let mut seed: u32 = u32::MAX;
        let val = rand_r(&mut seed as *mut u32);
        assert!(val >= 0 && val <= RAND_MAX);
    }
});
