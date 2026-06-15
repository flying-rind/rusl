use super::getsid;
use test_framework::test;

/// getsid(pid) 获取指定进程的会话 ID。
/// pid == 0 表示获取当前进程自身的会话 ID。
/// 成功返回会话 ID，失败返回 -1。

test!("test_getsid_self" {
        // 测试 1 — pid = 0（查询自身的会话 ID）
    {
        let sid = getsid(0);
        assert!(sid > 0, "getsid(0) 应返回当前进程的会话 ID（正整数）");
    }
});

test!("test_getsid_nonexistent_pid" {
        // 测试 2 — 使用不存在的 PID
    {
        let sid = getsid(0x7FFFFFFF);
        assert!(sid == -1, "getsid(不存在的PID) 应返回 -1");
    }
});

test!("test_getsid_negative_pid" {
        // 测试 3 — 使用负数 PID（边界条件）
    {
        let sid = getsid(-1);
        assert!(sid == -1, "getsid(-1) 应返回 -1");
    }
});

test!("test_getsid_self_pid_consistent" {
        // 测试 4 — 自身 PID 应获得一致的会话 ID
    {
        let pid = super::getpid();
        let sid1 = getsid(0);
        let sid2 = getsid(pid);
        assert_eq!(sid1, sid2, "getsid(0) 应等于 getsid(getpid())");
    }
});
