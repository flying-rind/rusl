use super::sleep;
use test_framework::test;

/// sleep(seconds) 使进程睡眠指定秒数。
/// 返回 0 表示完整睡眠，非零表示被信号中断后的剩余秒数。

test!("test_sleep_zero" {
        // 测试 1 — 睡眠 0 秒
    {
        let remaining = sleep(0);
        assert_eq!(remaining, 0, "sleep(0) 应返回 0");
    }
});

test!("test_sleep_one_second" {
        // 测试 2 — 睡眠 1 秒
    {
        let remaining = sleep(1);
        // 在正常环境下应完成完整睡眠返回 0
        assert_eq!(remaining, 0, "sleep(1) 应完整睡眠后返回 0");
    }
});

test!("test_sleep_multiple_zero" {
        // 测试 3 — 多次 sleep(0) 调用
    {
        let remaining = sleep(0);
        assert_eq!(remaining, 0, "第1次 sleep(0) 应返回 0");
        let remaining = sleep(0);
        assert_eq!(remaining, 0, "第2次 sleep(0) 应返回 0");
    }
});
