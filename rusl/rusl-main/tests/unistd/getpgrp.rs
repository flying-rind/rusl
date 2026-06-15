use super::getpgrp;
use test_framework::test;

/// getpgrp() 获取当前进程的进程组 ID。
/// 等价于 getpgid(0)，始终成功。

test!("test_getpgrp_returns_positive" {
        // 测试 1 — 基本调用
    {
        let pgrp = getpgrp();
        assert!(pgrp > 0, "getpgrp 应返回正整数 PGID");
    }
});

test!("test_getpgrp_eq_getpgid_self" {
        // 测试 2 — 与 getpgid(0) 一致性
    {
        let pgrp = getpgrp();
        let pgid = super::getpgid(0);
        assert_eq!(pgrp, pgid, "getpgrp() 应等于 getpgid(0)");
    }
});

test!("test_getpgrp_consistent" {
        // 测试 3 — 多次调用一致性
    {
        let pgrp1 = getpgrp();
        let pgrp2 = getpgrp();
        assert_eq!(pgrp1, pgrp2, "getpgrp 多次调用结果应一致");
    }
});
