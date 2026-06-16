//! 线程调度 API 集成测试
//! 测试函数: pthread_getschedparam, pthread_setschedparam, pthread_setschedprio,
//!            pthread_getconcurrency, pthread_setconcurrency, pthread_getcpuclockid

use super::*;
use test_framework::test;

// ============================================================================
// pthread_getschedparam 测试
// ============================================================================

test!("test_pthread_getschedparam_self" {
    // 获取当前线程的调度参数
    {
        let self_id = pthread_self();
        let mut policy: c_int = -1;
        let mut param: sched_param = sched_param { sched_priority: -1 };

        let ret = pthread_getschedparam(
            self_id,
            &mut policy as *mut c_int,
            &mut param as *mut sched_param,
        );
        assert_eq!(ret, 0, "getschedparam(self) 应返回 0");
        // policy 应为 SCHED_OTHER(0), SCHED_FIFO(1), SCHED_RR(2) 之一
        assert!(
            policy >= 0 && policy <= 2,
            "policy 应在 [0, 2] 范围内, got {}",
            policy
        );
        // 非实时线程的优先级为 0
        assert!(param.sched_priority >= 0, "sched_priority 应 >= 0");
    }
});


// ============================================================================
// pthread_setschedparam 测试
// ============================================================================

test!("test_pthread_setschedparam_no_privilege" {
    // 无 root 权限时 setschedparam 应返回 EPERM
    {
        let self_id = pthread_self();
        let param = sched_param { sched_priority: 0 };
        // 尝试设置 SCHED_OTHER 策略 (策略 0)
        let ret = pthread_setschedparam(self_id, 0, &param as *const sched_param);
        // 非特权用户可能返回 EPERM
        assert!(
            ret == 0 || ret == EPERM,
            "setschedparam 应返回 0 或 EPERM, got {}",
            ret
        );
    }
});

// ============================================================================
// pthread_setschedprio 测试
// ============================================================================

test!("test_pthread_setschedprio" {
    // 设置线程优先级
    {
        let ret = pthread_setschedprio(pthread_self(), 0);
        // 非特权用户可能返回 EPERM
        assert!(
            ret == 0 || ret == EPERM,
            "setschedprio 应返回 0 或 EPERM, got {}",
            ret
        );
    }
});

// ============================================================================
// pthread_getconcurrency / pthread_setconcurrency 测试
// ============================================================================

test!("test_pthread_getconcurrency" {
    // 获取并发级别, 默认应为 0
    {
        let level = pthread_getconcurrency();
        assert_eq!(level, 0, "默认 concurrency 应为 0");
    }
});

test!("test_pthread_setconcurrency" {
    // 设置并发级别 (Linux 上通常是 no-op，返回 EAGAIN)
    {
        let ret = pthread_setconcurrency(1);
        // Linux 上 setconcurrency 通常是 no-op，返回 EAGAIN
        assert!(
            ret == 0 || ret == EAGAIN,
            "setconcurrency 应返回 0 或 EAGAIN, got {}",
            ret
        );
    }
});

// ============================================================================
// pthread_getcpuclockid 测试
// ============================================================================

test!("test_pthread_getcpuclockid_self" {
    // 获取当前线程的 CPU 时钟 ID
    {
        let mut clock_id: clockid_t = -1;
        let ret = pthread_getcpuclockid(pthread_self(), &mut clock_id as *mut clockid_t);
        assert_eq!(ret, 0, "getcpuclockid(self) 应返回 0");
        // Linux 线程 CPU 时钟 ID 使用负值编码，只验证已写入
        assert_ne!(clock_id, -1, "clock_id 应已被修改, got {}", clock_id);
    }
});

