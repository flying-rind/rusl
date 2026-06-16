//! 条件变量 API 集成测试
//! 测试函数: pthread_cond_init, destroy, wait, timedwait, signal, broadcast, condattr_*

use super::*;
use test_framework::test;

// 多线程测试的共享上下文 (通过 arg 传递)
#[repr(C)]
struct CondTestCtx {
    mutex: *mut pthread_mutex_t,
    cond: *mut pthread_cond_t,
    signaled: AtomicBool,
}

extern "C" fn cond_waiter_thread(arg: *mut c_void) -> *mut c_void {
    if arg.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        let ctx = &*(arg as *const CondTestCtx);
        let ret = pthread_mutex_lock(ctx.mutex);
        if ret == 0 {
            let ret = pthread_cond_wait(ctx.cond, ctx.mutex);
            if ret == 0 {
                ctx.signaled.store(true, Ordering::SeqCst);
            }
            pthread_mutex_unlock(ctx.mutex);
        }
    }
    core::ptr::null_mut()
}

extern "C" fn cond_signaler_thread(arg: *mut c_void) -> *mut c_void {
    if arg.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        let ctx = &*(arg as *const CondTestCtx);
        let ret = pthread_mutex_lock(ctx.mutex);
        if ret == 0 {
            pthread_cond_signal(ctx.cond);
            pthread_mutex_unlock(ctx.mutex);
        }
    }
    core::ptr::null_mut()
}

// ============================================================================
// pthread_condattr 测试
// ============================================================================

test!("test_condattr_init_destroy" {
    // condattr init/destroy 基本功能
    {
        let mut attr: pthread_condattr_t = pthread_condattr_t { __attr: 0 };
        let ret = pthread_condattr_init(&mut attr as *mut pthread_condattr_t);
        assert_eq!(ret, 0, "condattr_init 应返回 0");

        let ret = pthread_condattr_destroy(&mut attr as *mut pthread_condattr_t);
        assert_eq!(ret, 0, "condattr_destroy 应返回 0");
    }
});

test!("test_condattr_set_get_clock" {
    // 设置/获取条件变量的时钟ID
    {
        let mut attr: pthread_condattr_t = pthread_condattr_t { __attr: 0 };
        let _ = pthread_condattr_init(&mut attr as *mut pthread_condattr_t);

        let ret = pthread_condattr_setclock(&mut attr as *mut pthread_condattr_t, 1); // CLOCK_MONOTONIC
        assert_eq!(ret, 0, "setclock 应返回 0");

        let mut clk: clockid_t = -1;
        let ret = pthread_condattr_getclock(
            &attr as *const pthread_condattr_t,
            &mut clk as *mut clockid_t,
        );
        assert_eq!(ret, 0, "getclock 应返回 0");
        assert_eq!(clk, 1, "clock 应为 CLOCK_MONOTONIC(1)");

        let _ = pthread_condattr_destroy(&mut attr as *mut pthread_condattr_t);
    }
});

test!("test_condattr_set_get_pshared" {
    // 设置/获取进程共享属性
    {
        let mut attr: pthread_condattr_t = pthread_condattr_t { __attr: 0 };
        let _ = pthread_condattr_init(&mut attr as *mut pthread_condattr_t);

        let ret = pthread_condattr_setpshared(
            &mut attr as *mut pthread_condattr_t,
            PTHREAD_PROCESS_SHARED,
        );
        assert_eq!(ret, 0, "setpshared 应返回 0");

        let mut pshared: c_int = -1;
        let ret = pthread_condattr_getpshared(
            &attr as *const pthread_condattr_t,
            &mut pshared as *mut c_int,
        );
        assert_eq!(ret, 0, "getpshared 应返回 0");
        assert_eq!(pshared, PTHREAD_PROCESS_SHARED, "pshared 应为 PROCESS_SHARED");

        let _ = pthread_condattr_destroy(&mut attr as *mut pthread_condattr_t);
    }
});

// ============================================================================
// pthread_cond 生命周期测试
// ============================================================================

test!("test_cond_init_default" {
    // 默认属性初始化条件变量
    {
        let mut cond: pthread_cond_t = unsafe { core::mem::zeroed() };
        let ret = pthread_cond_init(&mut cond as *mut pthread_cond_t, core::ptr::null());
        assert_eq!(ret, 0, "cond_init 应返回 0");

        let ret = pthread_cond_destroy(&mut cond as *mut pthread_cond_t);
        assert_eq!(ret, 0, "cond_destroy 应返回 0");
    }
});

// ============================================================================
// pthread_cond_signal / broadcast (无等待者)
// ============================================================================

test!("test_cond_signal_no_waiters" {
    // 无等待者时 signal 应返回 0
    {
        let mut cond: pthread_cond_t = unsafe { core::mem::zeroed() };
        let _ = pthread_cond_init(&mut cond as *mut pthread_cond_t, core::ptr::null());

        let ret = pthread_cond_signal(&mut cond as *mut pthread_cond_t);
        assert_eq!(ret, 0, "无等待者 signal 应返回 0");

        let _ = pthread_cond_destroy(&mut cond as *mut pthread_cond_t);
    }
});

test!("test_cond_broadcast_no_waiters" {
    // 无等待者时 broadcast 应返回 0
    {
        let mut cond: pthread_cond_t = unsafe { core::mem::zeroed() };
        let _ = pthread_cond_init(&mut cond as *mut pthread_cond_t, core::ptr::null());

        let ret = pthread_cond_broadcast(&mut cond as *mut pthread_cond_t);
        assert_eq!(ret, 0, "无等待者 broadcast 应返回 0");

        let _ = pthread_cond_destroy(&mut cond as *mut pthread_cond_t);
    }
});

// ============================================================================
// pthread_cond_timedwait 测试
// ============================================================================

test!("test_cond_timedwait_timeout" {
    // timedwait 超时应返回 ETIMEDOUT
    {
        let mut cond: pthread_cond_t = unsafe { core::mem::zeroed() };
        let mut mutex: pthread_mutex_t = unsafe { core::mem::zeroed() };
        let _ = pthread_cond_init(&mut cond as *mut pthread_cond_t, core::ptr::null());
        let _ = pthread_mutex_init(&mut mutex as *mut pthread_mutex_t, core::ptr::null());

        let _ = pthread_mutex_lock(&mut mutex as *mut pthread_mutex_t);

        let ts = timespec { tv_sec: 0, tv_nsec: 1000000 }; // 1ms
        let ret = pthread_cond_timedwait(
            &mut cond as *mut pthread_cond_t,
            &mut mutex as *mut pthread_mutex_t,
            &ts as *const timespec,
        );
        assert_eq!(ret, ETIMEDOUT, "timedwait 超时应返回 ETIMEDOUT");

        let _ = pthread_mutex_unlock(&mut mutex as *mut pthread_mutex_t);
        let _ = pthread_mutex_destroy(&mut mutex as *mut pthread_mutex_t);
        let _ = pthread_cond_destroy(&mut cond as *mut pthread_cond_t);
    }
});

// ============================================================================
// pthread_cond 多线程 signal 测试
// ============================================================================

test!("test_cond_wait_signal_multithread" {
    // 多线程: 一个线程等待, 另一个线程发送信号 (栈分配 + arg 传递)
    {
        let mut mutex: pthread_mutex_t = unsafe { core::mem::zeroed() };
        let mut cond: pthread_cond_t = unsafe { core::mem::zeroed() };
        let ret = pthread_mutex_init(&mut mutex as *mut pthread_mutex_t, core::ptr::null());
        assert_eq!(ret, 0);
        let ret = pthread_cond_init(&mut cond as *mut pthread_cond_t, core::ptr::null());
        assert_eq!(ret, 0);

        let ctx = CondTestCtx {
            mutex: &mut mutex as *mut pthread_mutex_t,
            cond: &mut cond as *mut pthread_cond_t,
            signaled: AtomicBool::new(false),
        };

        // 创建等待线程
        let mut waiter: pthread_t = core::ptr::null_mut();
        let ret = pthread_create(
            &mut waiter as *mut pthread_t,
            core::ptr::null(),
            Some(cond_waiter_thread),
            &raw const ctx as *mut c_void,
        );
        assert_eq!(ret, 0);

        // 创建发送信号线程
        let mut signaler: pthread_t = core::ptr::null_mut();
        let ret = pthread_create(
            &mut signaler as *mut pthread_t,
            core::ptr::null(),
            Some(cond_signaler_thread),
            &raw const ctx as *mut c_void,
        );
        assert_eq!(ret, 0);

        let mut result: *mut c_void = core::ptr::null_mut();
        let _ = pthread_join(waiter, &mut result as *mut *mut c_void);
        let _ = pthread_join(signaler, &mut result as *mut *mut c_void);

        assert!(ctx.signaled.load(Ordering::SeqCst), "等待线程应被 signal 唤醒");

        let _ = pthread_cond_destroy(&mut cond as *mut pthread_cond_t);
        let _ = pthread_mutex_destroy(&mut mutex as *mut pthread_mutex_t);
    }
});
