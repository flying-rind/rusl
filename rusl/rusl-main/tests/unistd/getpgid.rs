use super::getpgid;
use test_framework::test;

/// getpgid(pid) 获取指定进程的进程组 ID。
/// pid == 0 表示获取当前进程自身的进程组 ID。
/// 若进程不存在则返回 -1（errno = ESRCH）。

test!("test_getpgid_self" {
        // 测试 1 — pid = 0（查询自身的进程组 ID）
    {
        let pgid = getpgid(0);
        assert!(pgid > 0, "getpgid(0) 应返回当前进程的进程组 ID（正整数）");
    }
});

test!("test_getpgid_nonexistent_pid" {
        // 测试 2 — 使用无效 PID（不存在的进程）
    {
        // 使用一个极大的 PID，几乎不可能存在
        let pgid = getpgid(0x7FFFFFFF);
        assert!(pgid == -1, "getpgid(不存在的PID) 应返回 -1");
    }
});

test!("test_getpgid_negative_pid" {
        // 测试 3 — 使用负数 PID（边界条件）
    {
        let pgid = getpgid(-1);
        // 负数 PID 无效，应返回 -1
        assert!(pgid == -1, "getpgid(-1) 应返回 -1");
    }
});

test!("test_getpgid_self_pid_consistent" {
        // 测试 4 — 自身 PID 应获得一致的进程组 ID
    {
        let pid = super::getpid();
        let pgid1 = getpgid(0);
        let pgid2 = getpgid(pid);
        assert_eq!(pgid1, pgid2, "getpgid(0) 应等于 getpgid(getpid())");
    }
});
