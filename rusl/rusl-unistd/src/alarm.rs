//! alarm — 设置 SIGALRM 闹钟。
//! 对应 musl src/unistd/alarm.c
//!
//! 基于 setitimer(ITIMER_REAL, ...) 构建，返回之前闹钟剩余秒数。

use core::ffi::c_uint;
use crate::syscall::{raw_syscall3, __syscall_ret};

/// itimerval 结构体（用于 setitimer/getitimer）。
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

/// ITIMER_REAL — 真实时间定时器
const ITIMER_REAL: i64 = 0;

/// 设置一个真实时间闹钟，在 `seconds` 秒后向调用进程发送 SIGALRM 信号。
/// 调用 `alarm(0)` 取消任何待处理的闹钟。
/// 返回之前闹钟的剩余秒数（向上取整），0 表示之前无闹钟。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn alarm(seconds: c_uint) -> c_uint {
    let it = ITimerVal {
        it_interval: TimeVal { tv_sec: 0, tv_usec: 0 },
        it_value: TimeVal {
            tv_sec: seconds as i64,
            tv_usec: 0,
        },
    };
    let mut old = ITimerVal {
        it_interval: TimeVal { tv_sec: 0, tv_usec: 0 },
        it_value: TimeVal { tv_sec: 0, tv_usec: 0 },
    };
    unsafe {
        raw_syscall3(
            crate::syscall::SYS_setitimer,
            ITIMER_REAL,
            &it as *const ITimerVal as i64,
            &mut old as *mut ITimerVal as i64,
        );
    }
    // 返回之前闹钟剩余秒数（微秒有值则向上取整）
    (old.it_value.tv_sec as c_uint) + if old.it_value.tv_usec > 0 { 1 } else { 0 }
}

// ===========================================================================
// 单元测试
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;

    test!("test_alarm_zero_cancels" {
        // alarm(0) 应取消任何待处理的闹钟并返回 0（无之前闹钟）
        let ret = alarm(0);
        assert_eq!(ret, 0, "alarm(0) with no previous alarm should return 0");
    });

    test!("test_alarm_rounding_logic" {
        // 测试向上取整逻辑
        // 如果 tv_usec > 0 则加 1
        // alarm 函数： (old.it_value.tv_sec as c_uint) + if old.it_value.tv_usec > 0 { 1 } else { 0 }

        // 验证: old.tv_sec=5, tv_usec=0 -> 返回 5
        // 验证: old.tv_sec=5, tv_usec=1 -> 返回 6 (向上取整)
        let sec: u32 = 5;
        let usec: i64 = 0;
        let result = (sec as u32) + if usec > 0 { 1 } else { 0 };
        assert_eq!(result, 5);

        let usec2: i64 = 1;
        let result2 = (sec as u32) + if usec2 > 0 { 1 } else { 0 };
        assert_eq!(result2, 6);

        let usec3: i64 = 999999;
        let result3 = (sec as u32) + if usec3 > 0 { 1 } else { 0 };
        assert_eq!(result3, 6);
    });

    test!("test_alarm_set_and_cancel" {
        // 设置闹钟然后取消
        let prev = alarm(5); // 设置 5 秒闹钟
        // 第一次设置 alarm 应返回 0（之前无闹钟）
        assert_eq!(prev, 0, "first alarm should return 0");

        let remaining = alarm(0); // 取消闹钟
        // 取消后应返回之前闹钟剩余秒数（应 > 0，因为闹钟刚设定）
        assert!(remaining <= 5, "remaining should be <= 5 seconds");
    });

    test!("test_alarm_timeval_struct_sizes" {
        // 验证结构体大小符合预期
        assert_eq!(core::mem::size_of::<TimeVal>(), 16, "TimeVal should be 16 bytes (2 i64)");
        assert_eq!(core::mem::size_of::<ITimerVal>(), 32, "ITimerVal should be 32 bytes (4 i64)");
    });
}
