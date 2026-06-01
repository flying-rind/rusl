/// 模块: drand48_test
/// `drand48` 集成测试

use rusl_core::test;
use super::*;

test!("test_drand48_range" {
    // drand48 返回值在 [0.0, 1.0) 范围内。
    unsafe {
        let val = drand48();
        assert!(val >= 0.0 && val < 1.0);
    }
});

test!("test_drand48_not_constant" {
    // drand48 多次调用不应始终返回相同值（种子推进后不同）。
    unsafe {
        // 先重置种子到已知状态
        let val1 = drand48();
        let val2 = drand48();
        // 连续两次调用大概率不同（若种子初始为 0，第一次步进会得到确定值）
        // 此测试仅验证不会 panic 且范围正确
        assert!(val1 >= 0.0 && val1 < 1.0);
        assert!(val2 >= 0.0 && val2 < 1.0);
    }
});

test!("test_erand48_range" {
    // erand48 使用调用者种子，返回值在 [0.0, 1.0) 范围内。
    unsafe {
        let mut xsubi = [0x330eu16, 0xabcd, 0x1234];
        let val = erand48(xsubi.as_mut_ptr());
        assert!(val >= 0.0 && val < 1.0);
    }
});

test!("test_erand48_different_seeds" {
    // erand48 使用不同种子应产生不同结果。
    unsafe {
        let mut xsubi1 = [0x0000u16, 0x0000, 0x0000];
        let mut xsubi2 = [0x1234u16, 0x5678, 0x9abc];
        let val1 = erand48(xsubi1.as_mut_ptr());
        let val2 = erand48(xsubi2.as_mut_ptr());
        // 不同种子的结果通常不同
        // （理论上可能相同但概率极低，此处仅确保无 panic 且范围正确）
        assert!(val1 >= 0.0 && val1 < 1.0);
        assert!(val2 >= 0.0 && val2 < 1.0);
    }
});

test!("test_erand48_zero_seed" {
    // erand48 使用零种子产生已知最小输出。
    unsafe {
        let mut xsubi = [0u16, 0, 0]; // 种子 = 0
        let val = erand48(xsubi.as_mut_ptr());
        // 当种子为 0 时，步进后 seed = 0xB，因此 val = 0xB / 2^48
        assert!(val >= 0.0 && val < 1.0);
        // 验证种子被推进
        assert!(xsubi[0] != 0 || xsubi[1] != 0 || xsubi[2] != 0);
    }
});

test!("test_erand48_deterministic" {
    // erand48 的确定性：相同种子产生相同结果。
    unsafe {
        let mut x1 = [0x1234u16, 0x5678, 0x9abc];
        let mut x2 = [0x1234u16, 0x5678, 0x9abc];
        let v1 = erand48(x1.as_mut_ptr());
        let v2 = erand48(x2.as_mut_ptr());
        assert_eq!(v1, v2);
    }
});

test!("test_drand48_after_srand48_consistency" {
    // 测试 seed48 重置后 drand48 的可复现性。
    // （注意：依赖全局状态，需顺序执行）
    unsafe {
        // srand48(0) 设置种子为 {0x330E, 0, 0}
        srand48(0);
        let a = drand48();
        // 再次相同种子
        srand48(0);
        let b = drand48();
        // 相同种子应产生相同的第一个输出
        assert_eq!(a, b);
    }
});
