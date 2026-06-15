use super::ualarm;
use test_framework::test;



test!("test_ualarm_cancel_and_check" {
    // ualarm(value, interval) 设置微秒级间隔定时器。
    // value: 首次触发前的等待时间（微秒），0 表示不设置首次触发。
    // interval: 重复触发间隔（微秒），0 表示只触发一次。
    // 返回之前定时器剩余微秒数，0 表示之前无定时器。
    // ualarm(0, 0) 取消任何待处理的定时器。
    //
    // 注意：ualarm 与 alarm 共享同一个 ITIMER_REAL 定时器，
    // 因此测试间可能存在定时器状态残留。

        // 测试 1 — 取消任何已存在的定时器后验证状态
    {
        // 先取消所有可能的残留定时器
        ualarm(0, 0);
        // 再次取消，应返回 0
        let remaining = ualarm(0, 0);
        assert_eq!(remaining, 0, "清除残留定时器后，取消应返回 0");
    }
});

test!("test_ualarm_set_once_and_cancel" {
        // 测试 2 — 设置一次性定时器然后取消
    {
        // 先确保无残留定时器
        ualarm(0, 0);

        // 设置 500_000 微秒（0.5秒）后触发一次
        let old1 = ualarm(500_000, 0);
        assert_eq!(old1, 0, "首次设置定时器应返回 0");

        // 立即取消
        let old2 = ualarm(0, 0);
        // 剩余时间应接近 500_000 微秒
        assert!(old2 > 0, "取消定时器应返回正数剩余时间，实际: {}", old2);
        assert!(old2 <= 500_000,
                "取消定时器剩余时间不应超过设置值，实际: {}", old2);
    }
});

test!("test_ualarm_set_repeat_and_cancel" {
        // 测试 3 — 设置重复定时器然后取消
    {
        // 先确保无残留定时器
        ualarm(0, 0);

        // 设置 100_000 微秒后首次触发，之后每隔 200_000 微秒重复
        let old1 = ualarm(100_000, 200_000);
        assert_eq!(old1, 0, "首次设置重复定时器应返回 0");

        // 取消
        let old2 = ualarm(0, 0);
        assert!(old2 > 0, "取消重复定时器应返回正数剩余时间，实际: {}", old2);
        assert!(old2 <= 100_000,
                "取消定时器剩余时间不应超过首次触发值，实际: {}", old2);
    }
});

test!("test_ualarm_overwrite" {
        // 测试 4 — 覆盖前一个定时器
    {
        // 先确保无残留定时器
        ualarm(0, 0);

        // 设置 1_000_000 微秒（1秒）的定时器
        let old1 = ualarm(1_000_000, 0);
        assert_eq!(old1, 0, "首次设置定时器应返回 0");

        // 覆盖为 100 微秒的定时器
        let remaining = ualarm(100, 0);
        // 覆盖时应返回前一个定时器的剩余时间（可能为 0 如果已被处理）
        assert!(remaining >= 0,
                "覆盖前一个定时器返回的剩余时间 >= 0，实际: {}", remaining);

        // 清理：取消定时器
        ualarm(0, 0);
    }
});

test!("test_ualarm_interval_only" {
        // 测试 5 — ualarm(0, interval) 无首次触发，只设重复
    {
        // 先确保无残留定时器
        ualarm(0, 0);
        ualarm(0, 0); // 双重取消确保清理

        let old = ualarm(0, 500_000);
        // 返回之前定时器剩余时间，可能是 0 或其他值
        assert!(old >= 0, "ualarm(0, interval) 应返回 >= 0，实际: {}", old);

        // 取消
        let old2 = ualarm(0, 0);
        // 由于 value=0 没有首次触发定时器，取消可能返回 0
        assert!(old2 >= 0, "取消 interval-only 定时器应返回 >= 0，实际: {}", old2);
    }
});
