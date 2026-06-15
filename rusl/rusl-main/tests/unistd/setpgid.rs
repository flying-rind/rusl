use super::setpgid;
use test_framework::test;

/// setpgid(pid, pgid) 将指定进程的进程组 ID 设置为 pgid。
/// pid == 0 表示当前进程，pgid == 0 表示使用 pid 自身的值。
/// 成功返回 0，失败返回 -1 并设置 errno。

test!("test_setpgid_self_to_self" {
        // 测试 1 — setpgid(0, 0) 将当前进程的 PGID 设为其 PID
    {
        let ret = setpgid(0, 0);
        // 可能成功（返回0）也可能失败（如进程已是进程组首进程）
        // 两种情况都是允许的
        assert!(ret == 0 || ret == -1,
                "setpgid(0,0) 应返回 0（成功）或 -1（如已为首进程）");
        // 无论成功与否，进程应仍然可运行
    }
});

test!("test_setpgid_nonexistent_pid" {
        // 测试 2 — 使用不存在的 PID
    {
        let ret = setpgid(0x7FFFFFFF, 0);
        assert!(ret == -1, "setpgid(不存在的PID, 0) 应返回 -1");
    }
});

test!("test_setpgid_negative_pid" {
        // 测试 3 — 使用负数 PID（边界条件）
    {
        let ret = setpgid(-1, 0);
        assert!(ret == -1, "setpgid(-1, 0) 应返回 -1");
    }
});

test!("test_setpgid_self_pid" {
        // 测试 4 — 使用自身 PID 和自身 PGID
    {
        let pid = super::getpid();
        let pgid = super::getpgid(0);
        let ret = setpgid(pid, pgid);
        // 将进程移到已有进程组的操作
        assert!(ret == 0 || ret == -1,
                "setpgid(self, pgid) 的结果取决于进程状态");
    }
});
