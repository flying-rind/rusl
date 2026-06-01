/// 模块: lcong48_test
/// `lcong48` 集成测试

use super::*;
use rusl_core::test;

test!("test_lcong48_sets_params" {
    // lcong48 设置自定义参数后，lrand48 应产生确定性输出。
    unsafe {
        // 自定义参数：种子=0x1111_2222_3333, 乘数=0x4444_5555_6666, 加数=0x7
        let params: [u16; 7] = [0x3333, 0x2222, 0x1111, 0x6666, 0x5555, 0x4444, 0x0007];
        lcong48(params.as_ptr());

        // 通过 API 验证：相同参数产生相同序列
        let val1 = lrand48();

        lcong48(params.as_ptr());
        let val2 = lrand48();
        assert_eq!(val1, val2);
    }
});

test!("test_lcong48_drand48_no_panic" {
    // lcong48 后调用 drand48 不 panic。
    unsafe {
        let params = [1u16, 0, 0, 2, 0, 0, 3];
        lcong48(params.as_ptr());
        let val = drand48();
        assert!(val >= 0.0 && val < 1.0);
    }
});

test!("test_lcong48_all_zero" {
    // lcong48 设置所有零参数（边界情况）。
    unsafe {
        let params = [0u16; 7];
        lcong48(params.as_ptr());
        // 当乘数和加数都为 0 时，结果始终为 0.0
        let val = drand48();
        assert!(val >= 0.0 && val < 1.0);
    }
});

test!("test_lcong48_reproducibility" {
    // lcong48 的可复现性：相同参数 + srand48 重置种子后结果一致。
    unsafe {
        let params = [0x1234u16, 0x5678, 0x9abc, 0xe66d, 0xdeec, 0x5, 0xb];
        lcong48(params.as_ptr());
        let a = lrand48();

        // 再次设置相同参数
        lcong48(params.as_ptr());
        let b = lrand48();
        assert_eq!(a, b);
    }
});
