use super::usleep;
use test_framework::test;

/// usleep(useconds) 使进程睡眠指定微秒数。
/// 返回 0 表示成功，返回 -1 表示被信号中断或出错。

test!("test_usleep_zero" {
        // 测试 1 — 睡眠 0 微秒
    {
        let ret = usleep(0);
        assert_eq!(ret, 0, "usleep(0) 应返回 0");
    }
});

test!("test_usleep_one_microsecond" {
        // 测试 2 — 睡眠 1 微秒
    {
        let ret = usleep(1);
        // 在正常环境下应完成完整睡眠返回 0
        assert_eq!(ret, 0, "usleep(1) 应返回 0");
    }
});

test!("test_usleep_one_millisecond" {
        // 测试 3 — 睡眠 1000 微秒（1 毫秒）
    {
        let ret = usleep(1000);
        assert_eq!(ret, 0, "usleep(1000) 应返回 0");
    }
});

test!("test_usleep_one_second" {
        // 测试 4 — 睡眠 1_000_000 微秒（1 秒）
    {
        let ret = usleep(1_000_000);
        assert_eq!(ret, 0, "usleep(1_000_000) 应返回 0");
    }
});

test!("test_usleep_multiple_zero" {
        // 测试 5 — 多次调用
    {
        let ret1 = usleep(0);
        assert_eq!(ret1, 0, "第1次 usleep(0) 应返回 0");
        let ret2 = usleep(0);
        assert_eq!(ret2, 0, "第2次 usleep(0) 应返回 0");
    }
});
