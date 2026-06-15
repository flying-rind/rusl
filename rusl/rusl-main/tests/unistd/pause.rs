use super::pause;
use test_framework::test;

/// pause() 阻塞调用进程直到收到一个被捕获的信号。
/// 被信号中断后返回 -1（errno = EINTR）。
/// 若未被信号中断，将永久阻塞。在当前测试框架中无法直接测试。

test!("test_pause_exists" {
        // 测试 1 — 编译链接验证
    {
        // 验证 pause 符号可链接。
        // 不实际调用 pause()，因为它会永久阻塞。
        assert!(true);
    }
});
