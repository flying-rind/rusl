//! 读写锁 API 集成测试
//! 测试函数: pthread_rwlock_init, destroy, rdlock, wrlock, unlock, rwlockattr_*

use super::*;
use test_framework::test;

// ============================================================================
// pthread_rwlockattr 测试
// ============================================================================

test!("test_rwlockattr_init_destroy" {
    // rwlockattr init/destroy 基本功能
    {
        let mut attr: pthread_rwlockattr_t = pthread_rwlockattr_t { __attr: [0u32; 2] };
        let ret = pthread_rwlockattr_init(&mut attr as *mut pthread_rwlockattr_t);
        assert_eq!(ret, 0, "rwlockattr_init 应返回 0");

        let ret = pthread_rwlockattr_destroy(&mut attr as *mut pthread_rwlockattr_t);
        assert_eq!(ret, 0, "rwlockattr_destroy 应返回 0");
    }
});

test!("test_rwlockattr_set_get_pshared" {
    // 设置/获取进程共享属性
    {
        let mut attr: pthread_rwlockattr_t = pthread_rwlockattr_t { __attr: [0u32; 2] };
        let _ = pthread_rwlockattr_init(&mut attr as *mut pthread_rwlockattr_t);

        let ret = pthread_rwlockattr_setpshared(&mut attr as *mut pthread_rwlockattr_t, PTHREAD_PROCESS_SHARED);
        assert_eq!(ret, 0, "setpshared 应返回 0");

        let mut pshared: c_int = -1;
        let ret = pthread_rwlockattr_getpshared(
            &attr as *const pthread_rwlockattr_t,
            &mut pshared as *mut c_int,
        );
        assert_eq!(ret, 0, "getpshared 应返回 0");
        assert_eq!(pshared, PTHREAD_PROCESS_SHARED, "pshared 应为 PROCESS_SHARED");

        let _ = pthread_rwlockattr_destroy(&mut attr as *mut pthread_rwlockattr_t);
    }
});

// ============================================================================
// pthread_rwlock 生命周期测试
// ============================================================================

test!("test_rwlock_init_default" {
    // 默认属性初始化读写锁
    {
        let mut rwlock: pthread_rwlock_t = unsafe { core::mem::zeroed() };
        let ret = pthread_rwlock_init(&mut rwlock as *mut pthread_rwlock_t, core::ptr::null());
        assert_eq!(ret, 0, "rwlock_init 应返回 0");

        let ret = pthread_rwlock_destroy(&mut rwlock as *mut pthread_rwlock_t);
        assert_eq!(ret, 0, "rwlock_destroy 应返回 0");
    }
});

// ============================================================================
// pthread_rwlock rdlock/wrlock/unlock 测试
// ============================================================================

test!("test_rwlock_rdlock_unlock" {
    // 获取读锁并释放
    {
        let mut rwlock: pthread_rwlock_t = unsafe { core::mem::zeroed() };
        let _ = pthread_rwlock_init(&mut rwlock as *mut pthread_rwlock_t, core::ptr::null());

        let ret = pthread_rwlock_rdlock(&mut rwlock as *mut pthread_rwlock_t);
        assert_eq!(ret, 0, "rdlock 应返回 0");

        let ret = pthread_rwlock_unlock(&mut rwlock as *mut pthread_rwlock_t);
        assert_eq!(ret, 0, "unlock 应返回 0");

        let _ = pthread_rwlock_destroy(&mut rwlock as *mut pthread_rwlock_t);
    }
});

test!("test_rwlock_wrlock_unlock" {
    // 获取写锁并释放
    {
        let mut rwlock: pthread_rwlock_t = unsafe { core::mem::zeroed() };
        let _ = pthread_rwlock_init(&mut rwlock as *mut pthread_rwlock_t, core::ptr::null());

        let ret = pthread_rwlock_wrlock(&mut rwlock as *mut pthread_rwlock_t);
        assert_eq!(ret, 0, "wrlock 应返回 0");

        let ret = pthread_rwlock_unlock(&mut rwlock as *mut pthread_rwlock_t);
        assert_eq!(ret, 0, "unlock 应返回 0");

        let _ = pthread_rwlock_destroy(&mut rwlock as *mut pthread_rwlock_t);
    }
});

test!("test_rwlock_tryrdlock_unlocked" {
    // tryrdlock 未锁定的读写锁应成功
    {
        let mut rwlock: pthread_rwlock_t = unsafe { core::mem::zeroed() };
        let _ = pthread_rwlock_init(&mut rwlock as *mut pthread_rwlock_t, core::ptr::null());

        let ret = pthread_rwlock_tryrdlock(&mut rwlock as *mut pthread_rwlock_t);
        assert_eq!(ret, 0, "tryrdlock 应返回 0");

        let ret = pthread_rwlock_unlock(&mut rwlock as *mut pthread_rwlock_t);
        assert_eq!(ret, 0, "unlock 应返回 0");

        let _ = pthread_rwlock_destroy(&mut rwlock as *mut pthread_rwlock_t);
    }
});

test!("test_rwlock_trywrlock_unlocked" {
    // trywrlock 未锁定的读写锁应成功
    {
        let mut rwlock: pthread_rwlock_t = unsafe { core::mem::zeroed() };
        let _ = pthread_rwlock_init(&mut rwlock as *mut pthread_rwlock_t, core::ptr::null());

        let ret = pthread_rwlock_trywrlock(&mut rwlock as *mut pthread_rwlock_t);
        assert_eq!(ret, 0, "trywrlock 应返回 0");

        let ret = pthread_rwlock_unlock(&mut rwlock as *mut pthread_rwlock_t);
        assert_eq!(ret, 0, "unlock 应返回 0");

        let _ = pthread_rwlock_destroy(&mut rwlock as *mut pthread_rwlock_t);
    }
});

test!("test_rwlock_trywrlock_when_rdlocked" {
    // 读锁定时, trywrlock 应返回 EBUSY
    {
        let mut rwlock: pthread_rwlock_t = unsafe { core::mem::zeroed() };
        let _ = pthread_rwlock_init(&mut rwlock as *mut pthread_rwlock_t, core::ptr::null());

        // 先获取读锁
        let ret = pthread_rwlock_rdlock(&mut rwlock as *mut pthread_rwlock_t);
        assert_eq!(ret, 0, "rdlock 应成功");

        // trywrlock 应失败
        let ret = pthread_rwlock_trywrlock(&mut rwlock as *mut pthread_rwlock_t);
        assert_eq!(ret, EBUSY, "读锁定时 trywrlock 应返回 EBUSY");

        let _ = pthread_rwlock_unlock(&mut rwlock as *mut pthread_rwlock_t);
        let _ = pthread_rwlock_destroy(&mut rwlock as *mut pthread_rwlock_t);
    }
});

test!("test_rwlock_multiple_rdlock" {
    // 多个读锁可以同时持有
    {
        let mut rwlock: pthread_rwlock_t = unsafe { core::mem::zeroed() };
        let _ = pthread_rwlock_init(&mut rwlock as *mut pthread_rwlock_t, core::ptr::null());

        let ret1 = pthread_rwlock_rdlock(&mut rwlock as *mut pthread_rwlock_t);
        assert_eq!(ret1, 0, "第一次 rdlock 应成功");

        let ret2 = pthread_rwlock_rdlock(&mut rwlock as *mut pthread_rwlock_t);
        assert_eq!(ret2, 0, "第二次 rdlock (递归) 应成功或返回 0");

        let _ = pthread_rwlock_unlock(&mut rwlock as *mut pthread_rwlock_t);
        let _ = pthread_rwlock_unlock(&mut rwlock as *mut pthread_rwlock_t);
        let _ = pthread_rwlock_destroy(&mut rwlock as *mut pthread_rwlock_t);
    }
});


// ============================================================================
// pthread_rwlock timed 测试
// ============================================================================

test!("test_rwlock_timedrdlock_timeout" {
    // timedrdlock 超时应返回 ETIMEDOUT
    {
        let mut rwlock: pthread_rwlock_t = unsafe { core::mem::zeroed() };
        let _ = pthread_rwlock_init(&mut rwlock as *mut pthread_rwlock_t, core::ptr::null());

        // 先获取写锁
        let _ = pthread_rwlock_wrlock(&mut rwlock as *mut pthread_rwlock_t);

        let ts = timespec { tv_sec: 0, tv_nsec: 0 };
        let ret = pthread_rwlock_timedrdlock(&mut rwlock as *mut pthread_rwlock_t, &ts as *const timespec);
        assert_eq!(ret, ETIMEDOUT, "timedrdlock 超时应返回 ETIMEDOUT");

        let _ = pthread_rwlock_unlock(&mut rwlock as *mut pthread_rwlock_t);
        let _ = pthread_rwlock_destroy(&mut rwlock as *mut pthread_rwlock_t);
    }
});

test!("test_rwlock_timedwrlock_timeout" {
    // timedwrlock 超时应返回 ETIMEDOUT
    {
        let mut rwlock: pthread_rwlock_t = unsafe { core::mem::zeroed() };
        let _ = pthread_rwlock_init(&mut rwlock as *mut pthread_rwlock_t, core::ptr::null());

        // 先获取读锁
        let _ = pthread_rwlock_rdlock(&mut rwlock as *mut pthread_rwlock_t);

        let ts = timespec { tv_sec: 0, tv_nsec: 0 };
        let ret = pthread_rwlock_timedwrlock(&mut rwlock as *mut pthread_rwlock_t, &ts as *const timespec);
        assert_eq!(ret, ETIMEDOUT, "timedwrlock 超时应返回 ETIMEDOUT");

        let _ = pthread_rwlock_unlock(&mut rwlock as *mut pthread_rwlock_t);
        let _ = pthread_rwlock_destroy(&mut rwlock as *mut pthread_rwlock_t);
    }
});
