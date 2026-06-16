//! ualarm — 设置微秒级间隔定时器。
//! 对应 musl src/unistd/ualarm.c
//!
//! 基于 setitimer(ITIMER_REAL, ...) 构建，提供微秒精度和自动重复。

use core::ffi::c_uint;
use crate::syscall::raw_syscall3;

/// itimerval 结构体。
#[repr(C)]
struct ITimerVal {
    it_interval: TimeVal,
    it_value: TimeVal,
}

#[repr(C)]
struct TimeVal {
    tv_sec: i64,
    tv_usec: i64,
}

const ITIMER_REAL: i64 = 0;

/// 设置真实时间闹钟：在 `value` 微秒后首次触发 SIGALRM，
/// 之后每隔 `interval` 微秒重复触发。
/// 调用 `ualarm(0, 0)` 取消任何待处理的定时器。
/// 返回之前定时器剩余微秒数，0 表示之前无定时器。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn ualarm(value: c_uint, interval: c_uint) -> c_uint {
    let mut old = ITimerVal {
        it_interval: TimeVal { tv_sec: 0, tv_usec: 0 },
        it_value: TimeVal { tv_sec: 0, tv_usec: 0 },
    };
    let it = ITimerVal {
        it_interval: TimeVal {
            tv_sec: (interval / 1000000) as i64,
            tv_usec: (interval % 1000000) as i64,
        },
        it_value: TimeVal {
            tv_sec: (value / 1000000) as i64,
            tv_usec: (value % 1000000) as i64,
        },
    };
    unsafe {
        raw_syscall3(
            crate::syscall::SYS_setitimer,
            ITIMER_REAL,
            &it as *const ITimerVal as i64,
            &mut old as *mut ITimerVal as i64,
        );
    }
    // 返回之前定时器剩余微秒数
    (old.it_value.tv_sec as c_uint) * 1000000 + (old.it_value.tv_usec as c_uint)
}

// ===========================================================================
// 单元测试
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;

    test!("test_ualarm_cancel" {
        // ualarm(0, 0) 取消任何定时器，返回 0
        let ret = ualarm(0, 0);
        assert_eq!(ret, 0, "ualarm(0, 0) should cancel and return 0");
    });

    test!("test_ualarm_time_conversion" {
        // 验证微秒到 seconds+useconds 的转换
        // value=2_500_000 -> tv_sec=2, tv_usec=500000
        let v: c_uint = 2_500_000;
        let sec = (v / 1_000_000) as i64;
        let usec = (v % 1_000_000) as i64;
        assert_eq!(sec, 2);
        assert_eq!(usec, 500_000);
    });

    test!("test_ualarm_return_conversion" {
        // 验证返回值计算: old.tv_sec * 1000000 + old.tv_usec
        // old.tv_sec=3, tv_usec=500000 -> 3_500_000 usecs
        let sec: u32 = 3;
        let usec: u32 = 500_000;
        let result = sec * 1_000_000 + usec;
        assert_eq!(result, 3_500_000);
    });

    test!("test_ualarm_return_exact_second" {
        // old.tv_sec=5, tv_usec=0 -> 5_000_000 usecs
        let sec: u32 = 5;
        let usec: u32 = 0;
        let result = sec * 1_000_000 + usec;
        assert_eq!(result, 5_000_000);
    });

    test!("test_ualarm_interval_conversion" {
        // 验证 interval 参数转换
        let iv: c_uint = 1_500_000; // 1.5 秒
        let sec = (iv / 1_000_000) as i64;
        let usec = (iv % 1_000_000) as i64;
        assert_eq!(sec, 1);
        assert_eq!(usec, 500_000);
    });

    test!("test_ualarm_set_then_cancel" {
        // 设置 ualarm 然后取消
        let prev = ualarm(500_000, 0); // 0.5 秒后触发, 不重复
        assert_eq!(prev, 0, "first ualarm should return 0");
        let remaining = ualarm(0, 0); // 立即取消
        // 剩余时间应 <= 500000 微秒
        assert!(remaining <= 500_000, "remaining should be <= 500000 usecs");
    });
}
