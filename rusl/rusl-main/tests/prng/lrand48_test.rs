/// 模块: lrand48_test
/// `lrand48` 集成测试

use test_framework::test;
use super::*;

test!("test_lrand48_range" {
    // lrand48 返回值在 [0, 2^31) 范围内。
    {
        let val = lrand48();
        assert!(val >= 0 && val < (1i64 << 31));
    }
});

test!("test_lrand48_not_constant" {
    // lrand48 多次调用返回不同值。
    {
        let v1 = lrand48();
        let v2 = lrand48();
        assert!(v1 >= 0 && v1 < (1i64 << 31));
        assert!(v2 >= 0 && v2 < (1i64 << 31));
    }
});

test!("test_nrand48_range" {
    // nrand48 使用调用者种子，返回值在 [0, 2^31) 范围内。
    {
        let mut xsubi = [0x330eu16, 0xabcd, 0x1234];
        let val = nrand48(xsubi.as_mut_ptr());
        assert!(val >= 0 && val < (1i64 << 31));
    }
});

test!("test_nrand48_updates_seed" {
    // nrand48 推进了调用者种子。
    {
        let mut xsubi = [1u16, 0, 0];
        let _ = nrand48(xsubi.as_mut_ptr());
        // 种子应被推进（不等于初始值）
        assert!(xsubi[0] != 1 || xsubi[1] != 0 || xsubi[2] != 0);
    }
});

test!("test_nrand48_deterministic" {
    // nrand48 的确定性：相同种子产生相同结果。
    {
        let mut x1 = [0x1234u16, 0x5678, 0x9abc];
        let mut x2 = [0x1234u16, 0x5678, 0x9abc];
        let v1 = nrand48(x1.as_mut_ptr());
        let v2 = nrand48(x2.as_mut_ptr());
        assert_eq!(v1, v2);
    }
});

test!("test_lrand48_reproducibility" {
    // 验证 srand48 + lrand48 的可复现性。
    {
        srand48(12345);
        let a = lrand48();
        srand48(12345);
        let b = lrand48();
        assert_eq!(a, b);
    }
});

test!("test_nrand48_max_seed" {
    // nrand48 的最大种子值测试。
    {
        let mut xsubi = [0xffffu16, 0xffff, 0xffff]; // 48 位全 1
        let val = nrand48(xsubi.as_mut_ptr());
        assert!(val >= 0 && val < (1i64 << 31));
    }
});
