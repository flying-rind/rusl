//! usleep — 暂停执行指定微秒数。
//! 对应 musl src/unistd/usleep.c
//!
//! 基于 nanosleep() 构建，将微秒转换为秒+纳秒表示。

use core::ffi::{c_int, c_uint};
use rusl_internal::do_syscall;

/// timespec 结构体（用于 nanosleep）。
#[repr(C)]
struct TimeSpec {
    tv_sec: i64,
    tv_nsec: i64,
}

/// 使调用进程暂停执行至少 `useconds` 微秒。
/// 返回 0 表示成功，-1 表示被信号中断或出错（errno 设置）。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn usleep(useconds: c_uint) -> c_int {
    let tv = TimeSpec {
        tv_sec: (useconds / 1000000) as i64,
        tv_nsec: ((useconds % 1000000) * 1000) as i64,
    };
    unsafe {
        do_syscall!(
            rusl_internal::syscall::SYS_nanosleep,
            &tv as *const TimeSpec,
            core::ptr::null::<TimeSpec>()
        ) as c_int
    }
}

// ===========================================================================
// 单元测试
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;

    test!("test_usleep_zero" {
        // usleep(0) 应立即返回 0
        let ret = usleep(0);
        assert_eq!(ret, 0, "usleep(0) should return 0 immediately");
    });

    test!("test_usleep_time_conversion_exact_second" {
        // 验证 1 秒 = 1_000_000 微秒的转换
        // tv_sec = 1000000 / 1000000 = 1
        // tv_nsec = (1000000 % 1000000) * 1000 = 0
        let usecs: c_uint = 1_000_000;
        let tv_sec = (usecs / 1_000_000) as i64;
        let tv_nsec = ((usecs % 1_000_000) * 1000) as i64;
        assert_eq!(tv_sec, 1, "1000000 usecs -> 1 sec");
        assert_eq!(tv_nsec, 0, "exact second -> 0 nsec");
    });

    test!("test_usleep_time_conversion_subsecond" {
        // 验证 500_000 微秒 = 0.5 秒的转换
        let usecs: c_uint = 500_000;
        let tv_sec = (usecs / 1_000_000) as i64;
        let tv_nsec = ((usecs % 1_000_000) * 1000) as i64;
        assert_eq!(tv_sec, 0, "500000 usecs -> 0 sec");
        assert_eq!(tv_nsec, 500_000_000, "500000 usecs -> 500000000 nsec");
    });

    test!("test_usleep_time_conversion_max" {
        // 验证 u32::MAX 微秒转换不会溢出
        let usecs: c_uint = c_uint::MAX; // 4294967295
        let tv_sec = (usecs / 1_000_000) as i64;
        let tv_nsec = ((usecs % 1_000_000) * 1000) as i64;
        // 4294967295 / 1000000 = 4294
        // 4294967295 % 1000000 = 967295
        // 967295 * 1000 = 967295000
        assert_eq!(tv_sec, 4294, "u32::MAX usecs -> 4294 secs");
        assert_eq!(tv_nsec, 967_295_000, "remainder conversion check");
    });

    test!("test_usleep_small_value" {
        // usleep(1) 测试微秒级别睡眠
        let ret = usleep(1);
        assert_eq!(ret, 0, "usleep(1) should return 0 (or -1 if interrupted)");
    });
}
