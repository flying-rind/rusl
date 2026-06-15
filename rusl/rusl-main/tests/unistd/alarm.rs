use super::alarm;
use test_framework::test;

/// alarm(seconds) 设置 SIGALRM 定时器。
/// 返回之前闹钟的剩余秒数，0 表示之前无闹钟。
/// alarm(0) 取消任何待处理的闹钟，且不发送 SIGALRM。

test!("test_alarm_cancel_nonexistent" {
        // 测试 1 — 取消不存在的闹钟
    {
        let remaining = alarm(0);
        assert_eq!(remaining, 0, "取消不存在的闹钟应返回 0");
    }
});

test!("test_alarm_set_and_cancel" {
        // 测试 2 — 设置闹钟然后取消
    {
        // 设置一个 10 秒后的闹钟
        let old1 = alarm(10);
        // 之前无闹钟，返回 0
        assert_eq!(old1, 0, "首次设置闹钟应返回 0");

        // 立即取消闹钟
        let old2 = alarm(0);
        // 应返回约 10 秒（或略少，因为系统调用有时间开销）
        // 剩余时间应在 [1, 10] 范围内
        assert!(old2 >= 1 && old2 <= 10,
                "取消闹钟应返回约 10 秒的剩余时间，实际: {}", old2);
    }
});

test!("test_alarm_overwrite" {
        // 测试 3 — 覆盖前一个闹钟
    {
        // 设置 20 秒后的闹钟
        alarm(20);
        // 覆盖为 5 秒后的闹钟
        let remaining = alarm(5);
        // 应返回前一个闹钟的剩余秒数（约 20 秒）
        assert!(remaining >= 1 && remaining <= 20,
                "覆盖闹钟应返回前一个闹钟剩余时间，实际: {}", remaining);

        // 清理：取消闹钟
        alarm(0);
    }
});
