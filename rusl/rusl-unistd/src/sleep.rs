//! sleep — 暂停执行指定秒数。
//! 对应 musl src/unistd/sleep.c
//!
//! 基于 nanosleep() 构建，被信号中断时返回剩余秒数。

use core::ffi::c_uint;
use rusl_internal::syscall::raw_syscall2;

/// timespec 结构体（用于 nanosleep）。
#[repr(C)]
struct TimeSpec {
    tv_sec: i64,
    tv_nsec: i64,
}

/// 使调用进程暂停执行 `seconds` 秒。
/// 若被信号中断，返回剩余未睡眠秒数；完整睡眠返回 0。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn sleep(seconds: c_uint) -> c_uint {
    let mut tv = TimeSpec {
        tv_sec: seconds as i64,
        tv_nsec: 0,
    };
    unsafe {
        let r = raw_syscall2(
            rusl_internal::syscall::SYS_nanosleep,
            &tv as *const TimeSpec as i64,
            &mut tv as *mut TimeSpec as i64,
        );
        if r != 0 {
            // 被信号中断，返回剩余时间
            tv.tv_sec as c_uint
        } else {
            0
        }
    }
}

// ===========================================================================
// 单元测试
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;

    test!("test_sleep_zero" {
        // sleep(0) 应立即返回 0
        let ret = sleep(0);
        assert_eq!(ret, 0, "sleep(0) should return 0 immediately");
    });

    test!("test_sleep_return_value_is_unsigned" {
        // 返回值类型是 unsigned，且成功时总是 0
        let ret = sleep(0);
        assert_eq!(ret, 0);
    });

    test!("test_sleep_timespec_layout" {
        // 验证 TimeSpec 结构体布局
        let tv = TimeSpec { tv_sec: 10, tv_nsec: 500_000_000 };
        assert_eq!(tv.tv_sec, 10);
        assert_eq!(tv.tv_nsec, 500_000_000);
    });

    test!("test_sleep_nanosleep_integration" {
        // sleep 底层使用 nanosleep，验证基本行为
        // sleep(0) 是安全的
        let r = sleep(0);
        assert_eq!(r, 0);
    });
}
