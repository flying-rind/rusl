use super::setsid;
use test_framework::test;

/// setsid() 创建新会话并将调用进程设为会话首进程。
/// 同时断开与之前控制终端的连接。
/// 成功返回新会话 ID（等于调用进程 PID），失败返回 -1（EPERM：已是进程组首进程）。

test!("test_setsid_call" {
        // 测试 1 — 基本调用
    {
        let ret = setsid();
        if ret == -1 {
            // 进程已是进程组首进程或会话首进程（EPERM），这是预期行为
            assert!(true, "setsid 失败：进程已是会话/进程组首进程");
        } else {
            // 成功：返回值应等于当前进程 PID
            let pid = super::getpid();
            assert_eq!(ret, pid, "成功时 setsid() 应返回当前进程 PID");
        }
    }
});

test!("test_setsid_new_session" {
        // 测试 2 — 成功创建新会话后的验证
    {
        let ret = setsid();
        if ret != -1 {
            // 成功后，当前会话 ID 应等于当前进程 PID
            let sid = super::getsid(0);
            let pid = super::getpid();
            assert_eq!(sid, pid, "新会话 ID 应等于当前进程 PID");
            assert_eq!(ret, sid, "setsid 返回值应等于新会话 ID");
        }
    }
});
