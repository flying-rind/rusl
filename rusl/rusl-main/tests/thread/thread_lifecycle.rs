//! 线程生命周期 API 集成测试
//! 测试函数: pthread_create, pthread_join, pthread_detach, pthread_self, pthread_equal

use super::*;
use test_framework::test;

// 用于线程入口函数的共享原子变量
static THREAD_RAN: AtomicBool = AtomicBool::new(false);
static THREAD_RETURN_VALUE: AtomicI32 = AtomicI32::new(0);

// 简单线程入口: 设置标志并返回
extern "C" fn simple_thread_func(_arg: *mut c_void) -> *mut c_void {
    THREAD_RAN.store(true, Ordering::SeqCst);
    core::ptr::null_mut()
}

// 带返回值的线程入口
extern "C" fn return_value_thread_func(_arg: *mut c_void) -> *mut c_void {
    THREAD_RETURN_VALUE.store(42, Ordering::SeqCst);
    0xDEAD as *mut c_void
}

// 递增共享计数器的线程入口
extern "C" fn counter_thread_func(arg: *mut c_void) -> *mut c_void {
    if !arg.is_null() {
        unsafe {
            let val = &*(arg as *const AtomicI32);
            val.fetch_add(1, Ordering::SeqCst);
        }
    }
    core::ptr::null_mut()
}

// ============================================================================
// pthread_self / pthread_equal 测试
// ============================================================================

test!("test_pthread_self_not_null" {
    // pthread_self() 应返回非空指针
    {
        let self_id = pthread_self();
        assert!(!self_id.is_null(), "pthread_self() 应返回非空指针");
    }
});

test!("test_pthread_self_consistent" {
    // 多次调用 pthread_self() 应返回同一值
    {
        let id1 = pthread_self();
        let id2 = pthread_self();
        assert_eq!(pthread_equal(id1, id2), 1, "同一线程 self 应相等");
    }
});

test!("test_pthread_equal_same" {
    // pthread_equal 比较同一线程应返回非零
    {
        let self_id = pthread_self();
        assert_ne!(pthread_equal(self_id, self_id), 0, "相同线程应返回非零");
    }
});

test!("test_pthread_equal_null_null" {
    // 两个 NULL 应被视为相等 (或至少不崩溃)
    {
        let ret = pthread_equal(core::ptr::null_mut(), core::ptr::null_mut());
        // musl 中两个 NULL 指针比较相等
        assert_ne!(ret, 0, "两个 NULL pthread_t 应相等");
    }
});

test!("test_pthread_equal_diff" {
    // pthread_equal 比较 self 和 NULL 应返回 0
    {
        let self_id = pthread_self();
        let ret = pthread_equal(self_id, core::ptr::null_mut());
        assert_eq!(ret, 0, "self 和 NULL 应不相等");
    }
});

// ============================================================================
// pthread_create / pthread_join 测试
// ============================================================================

test!("test_pthread_create_and_join" {
    // 创建线程并 join, 验证线程已执行
    {
        THREAD_RAN.store(false, Ordering::SeqCst);

        let mut thread: pthread_t = core::ptr::null_mut();
        let ret = pthread_create(
            &mut thread as *mut pthread_t,
            core::ptr::null(),
            Some(simple_thread_func),
            core::ptr::null_mut(),
        );
        assert_eq!(ret, 0, "pthread_create 应返回 0");

        let mut result: *mut c_void = core::ptr::null_mut();
        let ret = pthread_join(thread, &mut result as *mut *mut c_void);
        assert_eq!(ret, 0, "pthread_join 应返回 0");

        assert!(THREAD_RAN.load(Ordering::SeqCst), "线程应已执行");
    }
});

test!("test_pthread_create_with_arg" {
    // 创建带参数的线程, 验证参数正确传递
    {
        let counter = AtomicI32::new(0);

        let mut thread: pthread_t = core::ptr::null_mut();
        let ret = pthread_create(
            &mut thread as *mut pthread_t,
            core::ptr::null(),
            Some(counter_thread_func),
            &counter as *const AtomicI32 as *mut c_void,
        );
        assert_eq!(ret, 0, "pthread_create 应返回 0");

        let mut result: *mut c_void = core::ptr::null_mut();
        let ret = pthread_join(thread, &mut result as *mut *mut c_void);
        assert_eq!(ret, 0, "pthread_join 应返回 0");

        assert_eq!(counter.load(Ordering::SeqCst), 1, "计数器应为 1");
    }
});

test!("test_pthread_create_return_value" {
    // 线程通过返回值返回数据, join 应能获取
    {
        THREAD_RETURN_VALUE.store(0, Ordering::SeqCst);

        let mut thread: pthread_t = core::ptr::null_mut();
        let ret = pthread_create(
            &mut thread as *mut pthread_t,
            core::ptr::null(),
            Some(return_value_thread_func),
            core::ptr::null_mut(),
        );
        assert_eq!(ret, 0, "pthread_create 应返回 0");

        let mut result: *mut c_void = core::ptr::null_mut();
        let ret = pthread_join(thread, &mut result as *mut *mut c_void);
        assert_eq!(ret, 0, "pthread_join 应返回 0");

        assert_eq!(THREAD_RETURN_VALUE.load(Ordering::SeqCst), 42, "线程内部逻辑应执行");
        assert_eq!(result, 0xDEAD as *mut c_void, "返回值应为 0xDEAD");
    }
});

test!("test_pthread_create_multiple" {
    // 创建多个线程, 验证全部正常执行
    {
        const N: usize = 3;
        let counter = AtomicI32::new(0);
        let mut threads: [pthread_t; N] = [core::ptr::null_mut(); N];

        for i in 0..N {
            let ret = pthread_create(
                &mut threads[i] as *mut pthread_t,
                core::ptr::null(),
                Some(counter_thread_func),
                &counter as *const AtomicI32 as *mut c_void,
            );
            assert_eq!(ret, 0);
        }

        for i in 0..N {
            let mut result: *mut c_void = core::ptr::null_mut();
            let ret = pthread_join(threads[i], &mut result as *mut *mut c_void);
            assert_eq!(ret, 0);
        }

        assert_eq!(counter.load(Ordering::SeqCst), N as i32, "计数器应为 N");
    }
});

// ============================================================================
// pthread_detach 测试
// ============================================================================

test!("test_pthread_detach" {
    // 创建线程并立即 detach
    {
        let mut thread: pthread_t = core::ptr::null_mut();
        let ret = pthread_create(
            &mut thread as *mut pthread_t,
            core::ptr::null(),
            Some(simple_thread_func),
            core::ptr::null_mut(),
        );
        assert_eq!(ret, 0, "pthread_create 应返回 0");

        // detach 分离线程
        let det_ret = pthread_detach(thread);
        assert_eq!(det_ret, 0, "pthread_detach 应返回 0");

        // detach 后不能再 join (EBUSY)
        // 注意: join 时已验证过不能 join
    }
});


// ============================================================================
// pthread_join 错误测试
// ============================================================================


// ============================================================================
// thrd_current / thrd_equal 测试
// ============================================================================

test!("test_thrd_current_not_null" {
    // thrd_current() 应返回非空指针
    {
        let self_id = thrd_current();
        assert!(!self_id.is_null(), "thrd_current() 应返回非空指针");
    }
});

test!("test_thrd_current_equals_pthread_self" {
    // thrd_current() 应等于 pthread_self()
    {
        let id1 = thrd_current();
        let id2 = pthread_self();
        assert_ne!(thrd_equal(id1, id2), 0, "thrd_current 应等于 pthread_self");
    }
});

test!("test_thrd_equal_c11" {
    // thrd_equal 等价于 pthread_equal
    {
        let self_id = thrd_current();
        assert_ne!(thrd_equal(self_id, self_id), 0, "thrd_equal 相同线程应返回非零");
        assert_eq!(thrd_equal(self_id, core::ptr::null_mut()), 0, "thrd_equal 不同应返回 0");
    }
});
