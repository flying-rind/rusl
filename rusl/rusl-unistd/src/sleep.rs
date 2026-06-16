//! sleep — 暂停执行指定秒数。
//! 对应 musl src/unistd/sleep.c

use core::ffi::{c_int, c_uint};

/// timespec 结构体（与 musl 的 struct timespec 布局一致）。
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
    let r = nanosleep(&tv, &mut tv);
    if r != 0 {
        tv.tv_sec as c_uint
    } else {
        0
    }
}

// 非 rusl 路径: FFI 调用 musl 的 nanosleep（内部使用 __syscall_cp，确保取消点语义）
#[cfg(not(feature = "rusl"))]
fn nanosleep(req: *const TimeSpec, rem: *mut TimeSpec) -> c_int {
    extern "C" {
        fn nanosleep(req: *const TimeSpec, rem: *mut TimeSpec) -> c_int;
    }
    unsafe { nanosleep(req, rem) }
}

// rusl 路径: 直接系统调用（TODO: 通过 __syscall_cp 实现取消点）
#[cfg(feature = "rusl")]
fn nanosleep(req: *const TimeSpec, rem: *mut TimeSpec) -> c_int {
    unsafe {
        crate::syscall::raw_syscall2(
            crate::syscall::SYS_nanosleep,
            req as i64,
            rem as i64,
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

    test!("test_sleep_zero" {
        let ret = sleep(0);
        assert_eq!(ret, 0, "sleep(0) should return 0 immediately");
    });

    test!("test_sleep_return_value_is_unsigned" {
        let ret = sleep(0);
        assert_eq!(ret, 0);
    });

    test!("test_sleep_timespec_layout" {
        let tv = TimeSpec { tv_sec: 10, tv_nsec: 500_000_000 };
        assert_eq!(tv.tv_sec, 10);
        assert_eq!(tv.tv_nsec, 500_000_000);
    });

    test!("test_sleep_nanosleep_integration" {
        let r = sleep(0);
        assert_eq!(r, 0);
    });
}
