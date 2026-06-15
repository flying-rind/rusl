use super::getpid;
use test_framework::test;

/// getpid() 获取当前进程 ID。
/// 该调用始终成功，返回调用进程的正整数 PID。

test!("test_getpid_returns_positive" {
        // 测试 1 — 基本调用
    {
        let pid = getpid();
        assert!(pid > 0, "getpid 应返回正整数 PID");
    }
});

test!("test_getpid_consistent" {
        // 测试 2 — 多次调用一致性
    {
        let pid1 = getpid();
        let pid2 = getpid();
        assert_eq!(pid1, pid2, "同一进程的 PID 应保持不变");
    }
});
