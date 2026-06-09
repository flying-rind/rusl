/// 模块: mrand48_test
/// `mrand48` 集成测试

use test_framework::test;
use super::*;

test!("test_mrand48_range" {
    // mrand48 返回值在 [-2^31, 2^31) 范围内。
    unsafe {
        let val = mrand48();
        assert!(val >= -(1i64 << 31) && val < (1i64 << 31));
    }
});

test!("test_mrand48_multiple_calls" {
    // mrand48 多次调用不 panic。
    unsafe {
        for _ in 0..10 {
            let val = mrand48();
            assert!(val >= -(1i64 << 31) && val < (1i64 << 31));
        }
    }
});

test!("test_jrand48_range" {
    // jrand48 使用调用者种子，返回值在 [-2^31, 2^31) 范围内。
    unsafe {
        let mut xsubi = [0x330eu16, 0xabcd, 0x1234];
        let val = jrand48(xsubi.as_mut_ptr());
        assert!(val >= -(1i64 << 31) && val < (1i64 << 31));
    }
});

test!("test_jrand48_updates_seed" {
    // jrand48 推进了调用者种子。
    unsafe {
        let mut xsubi = [1u16, 0, 0];
        let _ = jrand48(xsubi.as_mut_ptr());
        // 种子应被推进
        assert!(xsubi[0] != 1 || xsubi[1] != 0 || xsubi[2] != 0);
    }
});

test!("test_jrand48_deterministic" {
    // jrand48 的确定性：相同种子产生相同结果。
    unsafe {
        let mut x1 = [0x1234u16, 0x5678, 0x9abc];
        let mut x2 = [0x1234u16, 0x5678, 0x9abc];
        let v1 = jrand48(x1.as_mut_ptr());
        let v2 = jrand48(x2.as_mut_ptr());
        assert_eq!(v1, v2);
    }
});

test!("test_mrand48_reproducibility" {
    // 验证 srand48 + mrand48 的可复现性。
    unsafe {
        srand48(42);
        let a = mrand48();
        srand48(42);
        let b = mrand48();
        assert_eq!(a, b);
    }
});

test!("test_jrand48_zero_seed" {
    // jrand48 零种子测试。
    unsafe {
        let mut xsubi = [0u16; 3];
        let val = jrand48(xsubi.as_mut_ptr());
        assert!(val >= -(1i64 << 31) && val < (1i64 << 31));
    }
});

test!("test_jrand48_max_seed" {
    // jrand48 最大种子测试。
    unsafe {
        let mut xsubi = [0xffffu16; 3];
        let val = jrand48(xsubi.as_mut_ptr());
        assert!(val >= -(1i64 << 31) && val < (1i64 << 31));
    }
});
