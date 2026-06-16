//! 一次性初始化 API 集成测试
//! 测试函数: pthread_once, call_once

use super::*;
use test_framework::test;

static ONCE_COUNTER: AtomicI32 = AtomicI32::new(0);

extern "C" fn once_init_func() {
    ONCE_COUNTER.fetch_add(1, Ordering::SeqCst);
}

// ============================================================================
// pthread_once 测试
// ============================================================================

test!("test_pthread_once_single_call" {
    // 单次调用 pthread_once, init 应被执行一次
    {
        ONCE_COUNTER.store(0, Ordering::SeqCst);
        let mut once: pthread_once_t = PTHREAD_ONCE_INIT;

        let ret = pthread_once(&mut once as *mut pthread_once_t, Some(once_init_func));
        assert_eq!(ret, 0, "pthread_once 应返回 0");
        assert_eq!(ONCE_COUNTER.load(Ordering::SeqCst), 1, "init 应被执行一次");
    }
});

test!("test_pthread_once_no_repeat" {
    // 多次调用 pthread_once, init 只应执行一次
    {
        ONCE_COUNTER.store(0, Ordering::SeqCst);
        let mut once: pthread_once_t = PTHREAD_ONCE_INIT;

        let ret = pthread_once(&mut once as *mut pthread_once_t, Some(once_init_func));
        assert_eq!(ret, 0, "第一次 pthread_once 应返回 0");

        let ret = pthread_once(&mut once as *mut pthread_once_t, Some(once_init_func));
        assert_eq!(ret, 0, "第二次 pthread_once 应返回 0");

        assert_eq!(ONCE_COUNTER.load(Ordering::SeqCst), 1, "init 只应被执行一次");
    }
});

test!("test_pthread_once_multithread" {
    // 多线程并发调用 pthread_once, init 应只执行一次
    {
        ONCE_COUNTER.store(0, Ordering::SeqCst);
        let once: pthread_once_t = PTHREAD_ONCE_INIT;

        extern "C" fn once_thread_func(arg: *mut c_void) -> *mut c_void {
            let once_ptr = unsafe { &mut *(arg as *mut pthread_once_t) };
            let _ = pthread_once(once_ptr as *mut pthread_once_t, Some(once_init_func));
            core::ptr::null_mut()
        }

        const N: usize = 3;
        let mut threads: [pthread_t; N] = [core::ptr::null_mut(); N];

        for i in 0..N {
            let ret = pthread_create(
                &mut threads[i] as *mut pthread_t,
                core::ptr::null(),
                Some(once_thread_func),
                &raw const once as *mut c_void,
            );
            assert_eq!(ret, 0);
        }

        for i in 0..N {
            let mut result: *mut c_void = core::ptr::null_mut();
            let ret = pthread_join(threads[i], &mut result as *mut *mut c_void);
            assert_eq!(ret, 0);
        }

        assert_eq!(ONCE_COUNTER.load(Ordering::SeqCst), 1, "多线程调用 init 应只执行一次");
    }
});

// ============================================================================
// call_once (C11) 测试
// ============================================================================

test!("test_call_once_basic" {
    // call_once 基本功能
    {
        static C11_ONCE_COUNTER: AtomicI32 = AtomicI32::new(0);
        extern "C" fn c11_init_func() {
            C11_ONCE_COUNTER.fetch_add(1, Ordering::SeqCst);
        }

        C11_ONCE_COUNTER.store(0, Ordering::SeqCst);
        let mut flag: once_flag = ONCE_FLAG_INIT;

        call_once(&mut flag as *mut once_flag, Some(c11_init_func));
        // call_once 没有返回值 (void)

        assert_eq!(C11_ONCE_COUNTER.load(Ordering::SeqCst), 1, "C11 init 应被执行一次");

        // 再次调用不应再次执行
        call_once(&mut flag as *mut once_flag, Some(c11_init_func));
        assert_eq!(C11_ONCE_COUNTER.load(Ordering::SeqCst), 1, "C11 init 不应再次执行");
    }
});
