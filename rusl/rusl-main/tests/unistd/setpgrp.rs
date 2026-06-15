use super::setpgrp;
use test_framework::test;

/// setpgrp() 等价于 setpgid(0, 0)。
/// 将当前进程的进程组 ID 设置为其自身 PID。
/// musl 实现直接调用 setpgid(0, 0)，因此返回 setpgid 的返回值：
/// 成功返回 0，失败返回 -1。

test!("test_setpgrp_call" {
        // 测试 1 — 基本调用
    {
        let ret = setpgrp();
        // setpgid(0,0) 成功时返回 0，失败时返回 -1
        // 当进程已是进程组首进程时 setpgid(0,0) 可能失败
        assert!(ret == 0 || ret == -1,
                "setpgrp() 成功应返回 0，失败应返回 -1，实际: {}", ret);
    }
});

test!("test_setpgrp_eq_setpgid_zero" {
        // 测试 2 — 与 setpgid(0, 0) 一致性
    {
        let ret1 = setpgrp();
        let ret2 = super::setpgid(0, 0);
        // 两者语义等价，返回值相同
        assert_eq!(ret1, ret2, "setpgrp() 应等于 setpgid(0, 0)");
    }
});
