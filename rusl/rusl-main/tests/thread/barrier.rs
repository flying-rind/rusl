//! 屏障 API 集成测试
//! 测试函数: pthread_barrier_init, destroy, wait, barrierattr_*

use super::*;
use test_framework::test;

// 多线程屏障测试上下文
#[repr(C)]
struct BarrierTestCtx {
    barrier: *mut pthread_barrier_t,
    reached: AtomicI32,
}

extern "C" fn barrier_thread_func(arg: *mut c_void) -> *mut c_void {
    if arg.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        let ctx = &*(arg as *const BarrierTestCtx);
        ctx.reached.fetch_add(1, Ordering::SeqCst);
        let _ = pthread_barrier_wait(ctx.barrier);
    }
    core::ptr::null_mut()
}

// ============================================================================
// pthread_barrierattr 测试
// ============================================================================

test!("test_barrierattr_init_destroy" {
    // barrierattr init/destroy 基本功能
    {
        let mut attr: pthread_barrierattr_t = pthread_barrierattr_t { __attr: 0 };
        let ret = pthread_barrierattr_init(&mut attr as *mut pthread_barrierattr_t);
        assert_eq!(ret, 0, "barrierattr_init 应返回 0");

        let ret = pthread_barrierattr_destroy(&mut attr as *mut pthread_barrierattr_t);
        assert_eq!(ret, 0, "barrierattr_destroy 应返回 0");
    }
});

test!("test_barrierattr_set_get_pshared" {
    // 设置/获取进程共享属性
    {
        let mut attr: pthread_barrierattr_t = pthread_barrierattr_t { __attr: 0 };
        let _ = pthread_barrierattr_init(&mut attr as *mut pthread_barrierattr_t);

        let ret = pthread_barrierattr_setpshared(&mut attr as *mut pthread_barrierattr_t, PTHREAD_PROCESS_SHARED);
        assert_eq!(ret, 0, "setpshared 应返回 0");

        let mut pshared: c_int = -1;
        let ret = pthread_barrierattr_getpshared(
            &attr as *const pthread_barrierattr_t,
            &mut pshared as *mut c_int,
        );
        assert_eq!(ret, 0, "getpshared 应返回 0");
        assert_eq!(pshared, PTHREAD_PROCESS_SHARED, "pshared 应为 PROCESS_SHARED");

        let _ = pthread_barrierattr_destroy(&mut attr as *mut pthread_barrierattr_t);
    }
});

// ============================================================================
// pthread_barrier 生命周期测试
// ============================================================================

test!("test_barrier_init_destroy" {
    // 初始化/销毁屏障
    {
        let mut barrier: pthread_barrier_t = unsafe { core::mem::zeroed() };
        let ret = pthread_barrier_init(&mut barrier as *mut pthread_barrier_t, core::ptr::null(), 2);
        assert_eq!(ret, 0, "barrier_init 应返回 0");

        let ret = pthread_barrier_destroy(&mut barrier as *mut pthread_barrier_t);
        assert_eq!(ret, 0, "barrier_destroy 应返回 0");
    }
});

test!("test_barrier_init_count_zero" {
    // count=0 应返回 EINVAL
    {
        let mut barrier: pthread_barrier_t = unsafe { core::mem::zeroed() };
        let ret = pthread_barrier_init(&mut barrier as *mut pthread_barrier_t, core::ptr::null(), 0);
        assert_eq!(ret, EINVAL, "barrier_init(count=0) 应返回 EINVAL");
    }
});

// ============================================================================
// pthread_barrier_wait 测试
// ============================================================================

test!("test_barrier_wait_single_thread" {
    // 单线程 barrier_wait (count=1), 应直接返回
    {
        let mut barrier: pthread_barrier_t = unsafe { core::mem::zeroed() };
        let _ = pthread_barrier_init(&mut barrier as *mut pthread_barrier_t, core::ptr::null(), 1);

        let ret = pthread_barrier_wait(&mut barrier as *mut pthread_barrier_t);
        // 单线程时返回 PTHREAD_BARRIER_SERIAL_THREAD = -1
        assert_eq!(ret, -1, "单线程 barrier_wait 应返回 PTHREAD_BARRIER_SERIAL_THREAD");

        let _ = pthread_barrier_destroy(&mut barrier as *mut pthread_barrier_t);
    }
});

test!("test_barrier_wait_multithread" {
    // 多线程 barrier_wait, 所有线程到达后才继续 (栈分配 + arg 传递)
    {
        const N: usize = 2;
        let mut barrier: pthread_barrier_t = unsafe { core::mem::zeroed() };
        let ret = pthread_barrier_init(
            &mut barrier as *mut pthread_barrier_t,
            core::ptr::null(),
            N as c_uint,
        );
        assert_eq!(ret, 0);

        let ctx = BarrierTestCtx {
            barrier: &mut barrier as *mut pthread_barrier_t,
            reached: AtomicI32::new(0),
        };

        let mut threads: [pthread_t; N] = [core::ptr::null_mut(); N];

        for i in 0..N {
            let ret = pthread_create(
                &mut threads[i] as *mut pthread_t,
                core::ptr::null(),
                Some(barrier_thread_func),
                &raw const ctx as *mut c_void,
            );
            assert_eq!(ret, 0);
        }

        for i in 0..N {
            let mut result: *mut c_void = core::ptr::null_mut();
            let ret = pthread_join(threads[i], &mut result as *mut *mut c_void);
            assert_eq!(ret, 0);
        }
    }
});
