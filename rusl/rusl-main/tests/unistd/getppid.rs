use super::getppid;
use test_framework::test;

/// getppid() 获取当前进程的父进程 ID。
/// 该调用始终成功，返回父进程的正整数 PID。

test!("test_getppid_returns_positive" {
        // 测试 1 — 基本调用
    {
        let ppid = getppid();
        assert!(ppid > 0, "getppid 应返回正整数 PPID");
    }
});

test!("test_getppid_consistent" {
        // 测试 2 — 多次调用一致性
    {
        let ppid1 = getppid();
        let ppid2 = getppid();
        assert_eq!(ppid1, ppid2, "同一进程的 PPID 应保持不变");
    }
});
