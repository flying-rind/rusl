//! 信号量 API 集成测试
//! 测试函数: sem_init, sem_destroy, sem_getvalue, sem_wait, sem_trywait, sem_timedwait, sem_post

use super::*;
use test_framework::test;

// 多线程信号量测试上下文
#[repr(C)]
struct SemTestCtx {
    sem: *mut sem_t,
    poster_done: AtomicBool,
}

extern "C" fn poster_func(arg: *mut c_void) -> *mut c_void {
    if arg.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        let ctx = &*(arg as *const SemTestCtx);
        let ret = sem_post(ctx.sem);
        if ret == 0 {
            ctx.poster_done.store(true, Ordering::SeqCst);
        }
    }
    core::ptr::null_mut()
}

extern "C" fn waiter_func(arg: *mut c_void) -> *mut c_void {
    if arg.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        let ctx = &*(arg as *const SemTestCtx);
        let _ = sem_wait(ctx.sem);
    }
    core::ptr::null_mut()
}

// ============================================================================
// sem_init / sem_destroy 测试
// ============================================================================

test!("test_sem_init_destroy" {
    // 初始化/销毁匿名信号量
    {
        let mut sem: sem_t = unsafe { core::mem::zeroed() };
        let ret = sem_init(&mut sem as *mut sem_t, 0, 1);
        assert_eq!(ret, 0, "sem_init 应返回 0");

        let ret = sem_destroy(&mut sem as *mut sem_t);
        assert_eq!(ret, 0, "sem_destroy 应返回 0");
    }
});

test!("test_sem_init_pshared" {
    // 进程共享信号量 (0=进程私有)
    {
        let mut sem: sem_t = unsafe { core::mem::zeroed() };
        let ret = sem_init(&mut sem as *mut sem_t, 0, 0);
        assert_eq!(ret, 0, "sem_init(pshared=0) 应返回 0");

        let _ = sem_destroy(&mut sem as *mut sem_t);
    }
});

test!("test_sem_init_overflow" {
    // 初始值超过 SEM_VALUE_MAX 应返回 -1 (errno=EINVAL)
    {
        let mut sem: sem_t = unsafe { core::mem::zeroed() };
        let ret = sem_init(
            &mut sem as *mut sem_t,
            0,
            (SEM_VALUE_MAX as c_uint) + 1,
        );
        assert_eq!(ret, -1, "sem_init(value > SEM_VALUE_MAX) 应返回 -1");
    }
});

// ============================================================================
// sem_post / sem_wait / sem_trywait 测试
// ============================================================================

test!("test_sem_wait_post" {
    // sem_wait 递减, sem_post 递增
    {
        let mut sem: sem_t = unsafe { core::mem::zeroed() };
        let _ = sem_init(&mut sem as *mut sem_t, 0, 1);

        let ret = sem_wait(&mut sem as *mut sem_t);
        assert_eq!(ret, 0, "sem_wait 应返回 0");

        let ret = sem_post(&mut sem as *mut sem_t);
        assert_eq!(ret, 0, "sem_post 应返回 0");

        let _ = sem_destroy(&mut sem as *mut sem_t);
    }
});

test!("test_sem_trywait_success" {
    // trywait 信号量值 > 0 时应成功
    {
        let mut sem: sem_t = unsafe { core::mem::zeroed() };
        let _ = sem_init(&mut sem as *mut sem_t, 0, 1);

        let ret = sem_trywait(&mut sem as *mut sem_t);
        assert_eq!(ret, 0, "sem_trywait(val=1) 应返回 0");

        let _ = sem_destroy(&mut sem as *mut sem_t);
    }
});

test!("test_sem_trywait_fail" {
    // trywait 信号量值为 0 时应返回 -1 (errno=EAGAIN)
    {
        let mut sem: sem_t = unsafe { core::mem::zeroed() };
        let _ = sem_init(&mut sem as *mut sem_t, 0, 0);

        let ret = sem_trywait(&mut sem as *mut sem_t);
        assert_eq!(ret, -1, "sem_trywait(val=0) 应返回 -1");

        let _ = sem_destroy(&mut sem as *mut sem_t);
    }
});

test!("test_sem_post_multiple" {
    // 多次 sem_post
    {
        let mut sem: sem_t = unsafe { core::mem::zeroed() };
        let _ = sem_init(&mut sem as *mut sem_t, 0, 0);

        let ret = sem_post(&mut sem as *mut sem_t);
        assert_eq!(ret, 0, "第一次 sem_post 应返回 0");

        let ret = sem_post(&mut sem as *mut sem_t);
        assert_eq!(ret, 0, "第二次 sem_post 应返回 0");

        let mut val: c_int = -1;
        let ret = sem_getvalue(&mut sem as *mut sem_t, &mut val as *mut c_int);
        assert_eq!(ret, 0, "sem_getvalue 应返回 0");
        assert_eq!(val, 2, "信号量值应为 2");

        let _ = sem_destroy(&mut sem as *mut sem_t);
    }
});

// ============================================================================
// sem_getvalue 测试
// ============================================================================

test!("test_sem_getvalue_initial" {
    // 获取信号量初始值
    {
        let mut sem: sem_t = unsafe { core::mem::zeroed() };
        let _ = sem_init(&mut sem as *mut sem_t, 0, 5);

        let mut val: c_int = -1;
        let ret = sem_getvalue(&mut sem as *mut sem_t, &mut val as *mut c_int);
        assert_eq!(ret, 0, "sem_getvalue 应返回 0");
        assert_eq!(val, 5, "初始值为 5");

        let _ = sem_destroy(&mut sem as *mut sem_t);
    }
});

test!("test_sem_getvalue_after_wait" {
    // wait 后获取值
    {
        let mut sem: sem_t = unsafe { core::mem::zeroed() };
        let _ = sem_init(&mut sem as *mut sem_t, 0, 3);

        let _ = sem_wait(&mut sem as *mut sem_t);

        let mut val: c_int = -1;
        let ret = sem_getvalue(&mut sem as *mut sem_t, &mut val as *mut c_int);
        assert_eq!(ret, 0, "sem_getvalue 应返回 0");
        assert_eq!(val, 2, "wait 后值应为 2");

        let _ = sem_destroy(&mut sem as *mut sem_t);
    }
});

// ============================================================================
// sem_timedwait 测试
// ============================================================================

test!("test_sem_timedwait_timeout" {
    // timedwait 超时应返回 -1 (errno=ETIMEDOUT)
    {
        let mut sem: sem_t = unsafe { core::mem::zeroed() };
        let _ = sem_init(&mut sem as *mut sem_t, 0, 0);

        let ts = timespec { tv_sec: 0, tv_nsec: 0 };
        let ret = sem_timedwait(&mut sem as *mut sem_t, &ts as *const timespec);
        assert_eq!(ret, -1, "sem_timedwait 超时应返回 -1");

        let _ = sem_destroy(&mut sem as *mut sem_t);
    }
});

// ============================================================================
// 信号量多线程测试
// ============================================================================

test!("test_sem_multithread_post_wait" {
    // 多线程: 一个线程 post, 另一个线程 wait (栈分配 + arg 传递)
    {
        let mut sem: sem_t = unsafe { core::mem::zeroed() };
        let ret = sem_init(&mut sem as *mut sem_t, 0, 0);
        assert_eq!(ret, 0);

        let ctx = SemTestCtx {
            sem: &mut sem as *mut sem_t,
            poster_done: AtomicBool::new(false),
        };

        // 创建 waiter 线程 (会阻塞在 wait 上)
        let mut waiter: pthread_t = core::ptr::null_mut();
        let ret = pthread_create(
            &mut waiter as *mut pthread_t,
            core::ptr::null(),
            Some(waiter_func),
            &raw const ctx as *mut c_void,
        );
        assert_eq!(ret, 0);

        // 创建 poster 线程
        let mut poster: pthread_t = core::ptr::null_mut();
        let ret = pthread_create(
            &mut poster as *mut pthread_t,
            core::ptr::null(),
            Some(poster_func),
            &raw const ctx as *mut c_void,
        );
        assert_eq!(ret, 0);

        let mut result: *mut c_void = core::ptr::null_mut();
        let _ = pthread_join(poster, &mut result as *mut *mut c_void);
        let _ = pthread_join(waiter, &mut result as *mut *mut c_void);

        assert!(ctx.poster_done.load(Ordering::SeqCst), "poster 应已完成");

        let _ = sem_destroy(&mut sem as *mut sem_t);
    }
});
