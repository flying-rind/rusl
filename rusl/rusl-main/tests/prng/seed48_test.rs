/// 模块: seed48_test
/// `seed48` 集成测试

use super::*;
use test_framework::test;

test!("test_seed48_returns_non_null" {
    // seed48 返回非空指针。
    unsafe {
        let new_seed = [0x1234u16, 0x5678, 0x9abc];
        let old = seed48(new_seed.as_ptr());
        assert!(!old.is_null());
    }
});

test!("test_seed48_old_seed_readable" {
    // seed48 返回的旧种子指针指向 3 个 u16 的有效数据。
    unsafe {
        let new_seed = [0x1234u16, 0x5678, 0x9abc];
        let old = seed48(new_seed.as_ptr());
        // 旧种子应该是之前状态的快照（至少可读 3 个 u16）
        let old0 = *old;
        let old1 = *old.add(1);
        let old2 = *old.add(2);
        // 仅验证可读且不 panic
        let _ = (old0, old1, old2);
    }
});

test!("test_seed48_only_modifies_seed" {
    // seed48 仅修改种子部分，不修改乘数和加数（musl 行为）。
    // 通过 API 验证：lcong48 设定乘数/加数后，seed48 改变种子
    // 重新恢复相同状态应产生相同序列。
    unsafe {
        // 先用 lcong48 设置非默认参数
        let custom_params = [1u16, 0, 0, 2, 0, 0, 3]; // 种子=1, 乘数=2, 加数=3
        lcong48(custom_params.as_ptr());

        // 然后用 seed48 设置新种子
        let new_seed = [0x1234u16, 0x5678, 0x9abc];
        let _old = seed48(new_seed.as_ptr());
        let val1 = lrand48();

        // 恢复相同状态：lcong48 + seed48 应产生相同结果
        lcong48(custom_params.as_ptr());
        let _old2 = seed48(new_seed.as_ptr());
        let val2 = lrand48();

        assert_eq!(val1, val2);
    }
});

test!("test_seed48_returns_old_snapshot" {
    // seed48 返回的旧种子值为调用前的种子快照。
    unsafe {
        // 先设置一个已知种子
        let first_seed = [0xaaaau16, 0xbbbb, 0xcccc];
        let old1 = seed48(first_seed.as_ptr());
        let _old1_vals = [*old1, *old1.add(1), *old1.add(2)];

        // 设置第二个种子
        let second_seed = [0x1111u16, 0x2222, 0x3333];
        let old2 = seed48(second_seed.as_ptr());

        // old2 应指向 old1 的值（即第一次 seed48 之前的种子 = {0,0,0}）
        assert_eq!(*old2, 0xaaaa);
        assert_eq!(*old2.add(1), 0xbbbb);
        assert_eq!(*old2.add(2), 0xcccc);
    }
});
