use core::ffi::c_int;
use super::nice;
use test_framework::test;

/// nice(inc) 将调用进程的调度优先级增加 inc。
/// nice 值范围 [-NZERO, NZERO-1] 即 [-20, 19]。
/// 返回新的 nice 值。

test!("test_nice_query" {
        // 测试 1 — inc = 0（查询当前 nice 值）
    {
        let n = nice(0);
        // nice 值应在有效范围 [-20, 19] 内
        assert!(n >= -20 && n <= 19,
                "当前 nice 值应在 [-20, 19] 范围内，实际: {}", n);
    }
});

test!("test_nice_increase" {
        // 测试 2 — inc = 1（增加 nice 值，降低优先级）
    {
        let old = nice(0);
        let new = nice(1);
        // 非特权用户增加 nice 值应成功，新值 >= 旧值
        // 注意：nice 值可能被裁剪到上限
        assert!(new >= old || new == 19,
                "增加 nice 值后新值应 >= 旧值（或已达上限 19），旧={}, 新={}", old, new);
    }
});

test!("test_nice_roundtrip" {
        // 测试 3 — 恢复原始 nice 值
    {
        let original = nice(0);
        // 增加 1
        let _increased = nice(1);
        // 减少 1（尝试恢复，但非特权用户可能被忽略）
        let restored = nice(-1);
        // 验证：返回值在有效范围内
        assert!(restored >= -20 && restored <= 19,
                "nice(-1) 返回值应在 [-20, 19]，实际: {}", restored);
        // 恢复原始值
        // 注意：非特权用户可能无法降低 nice 值
        let _ = nice(original - nice(0));
    }
});

test!("test_nice_large_positive" {
        // 测试 4 — 边界条件：极大增量
    {
        let _old = nice(0);
        let new = nice(100);
        // nice 值不应超过上限 19
        assert!(new <= 19, "nice(100) 结果不应超过 19，实际: {}", new);
        assert!(new >= -20, "nice(100) 结果不应小于 -20，实际: {}", new);
    }
});

test!("test_nice_large_negative" {
        // 测试 5 — 边界条件：极大负增量
    {
        let _old = nice(0);
        let new = nice(-100);
        // nice 值不应低于下限 -20（但非特权用户请求可能被忽略）
        assert!(new >= -20, "nice(-100) 结果不应小于 -20，实际: {}", new);
        assert!(new <= 19, "nice(-100) 结果不应超过 19，实际: {}", new);
    }
});
