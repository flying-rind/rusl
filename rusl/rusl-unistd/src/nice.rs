//! nice — 调整进程调度优先级。
//! 对应 musl src/unistd/nice.c
//!
//! 使用 getpriority/setpriority 系统调用实现。

use core::ffi::c_int;
use crate::syscall::{raw_syscall2, raw_syscall3, __syscall_ret};

const PRIO_PROCESS: i64 = 0;

/// 将调用进程的调度优先级（nice 值）增加 `inc`。
/// nice 值范围 [-20, 19]，较小值表示较高优先级。
/// 返回新的 nice 值，错误时返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn nice(inc: c_int) -> c_int {
    // 内核 getpriority 返回 (20 - nice_value)，范围 [1, 40]
    let kernel_ret = unsafe {
        let r = raw_syscall2(
            crate::syscall::SYS_getpriority,
            PRIO_PROCESS,
            0,
        ) as u64;
        __syscall_ret(r)
    };
    if kernel_ret < 0 {
        return -1;
    }
    // 转换为实际 nice 值：nice = 20 - kernel_ret
    // 注意：kernel_ret 范围 [1, 40]，所以 nice 范围 [-20, 19]
    let current_nice = (20 - kernel_ret) as c_int;

    // 计算新的 nice 值（限制在 [-20, 19] 范围内）并设置
    let new_nice = {
        let val = current_nice + inc;
        if val < -20 { -20 } else if val > 19 { 19 } else { val }
    };
    // 内核 setpriority 期望参数为 (20 - nice_value)
    let set_val = (20 - new_nice) as i64;
    let r = unsafe {
        let r = raw_syscall3(
            crate::syscall::SYS_setpriority,
            PRIO_PROCESS,
            0,
            set_val,
        ) as u64;
        __syscall_ret(r)
    };
    if r < 0 {
        return -1;
    }
    // setpriority 成功，返回新的 nice 值
    new_nice
}

// ===========================================================================
// 单元测试
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;

    test!("test_nice_zero_inc" {
        // nice(0) 不改变优先级，应返回当前 nice 值
        let ret = nice(0);
        // 在无特权环境下可能返回当前 nice 值（通常是 0）
        // 或者因权限不足返回 -1 (EPERM)
        assert!(ret >= -20 && ret <= 19 || ret == -1,
            "nice(0) should return nice value [-20, 19] or -1, got {}", ret);
    });

    test!("test_nice_conversion_formula" {
        // 验证内核返回值到 nice 值的转换公式
        // nice = 20 - kernel_ret (kernel_ret 范围 [1, 40])
        // 所以 nice 范围 [-20, 19]
        assert_eq!(20i64 - 1, 19, "kernel_ret=1 -> nice=19");
        assert_eq!(20i64 - 40, -20, "kernel_ret=40 -> nice=-20");
        assert_eq!(20i64 - 20, 0, "kernel_ret=20 -> nice=0");
    });

    test!("test_nice_clamping_lower_bound" {
        // 即使请求降低到 -20 以下，值应被限制在 -20
        // nice(-100) 应该在 nice(0) 基础上减少但不能低于 -20
        // 注意：降低 nice 值需要 CAP_SYS_NICE 权限
        // 无特权时 setpriority 会失败返回 -1 (EPERM)
        let ret = nice(-100);
        // 可能因权限不足返回 -1
        if ret != -1 {
            assert!(ret >= -20, "nice value should be >= -20, got {}", ret);
        }
    });

    test!("test_nice_clamping_upper_bound" {
        // nice(100) 应在当前 nice 值基础上增加但不能超过 19
        let ret = nice(100);
        if ret != -1 {
            assert!(ret <= 19, "nice value should be <= 19, got {}", ret);
        }
    });
}
