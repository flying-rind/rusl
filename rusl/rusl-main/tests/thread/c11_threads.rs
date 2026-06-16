//! C11 线程 API 集成测试
//! 测试函数: call_once, cnd_*, mtx_*, thrd_*, tss_*
//! (call_once 测试已在 once.rs 中覆盖, 此处不再重复)

use super::*;
use test_framework::test;

// thrd_create 线程入口 (返回 int)
extern "C" fn c11_thread_func(_arg: *mut c_void) -> c_int {
    0 // thrd_success
}

extern "C" fn c11_thread_func_with_value(_arg: *mut c_void) -> c_int {
    42
}

// ============================================================================
// cnd_* 条件变量测试
// ============================================================================

test!("test_cnd_init_destroy" {
    // 初始化/销毁 C11 条件变量
    {
        let mut cond: cnd_t = unsafe { core::mem::zeroed() };
        let ret = cnd_init(&mut cond as *mut cnd_t);
        assert_eq!(ret, 0, "cnd_init 应返回 0");

        cnd_destroy(&mut cond as *mut cnd_t);
        // cnd_destroy 返回 void
    }
});

test!("test_cnd_signal_broadcast_no_waiters" {
    // 无等待者时 signal/broadcast 应成功
    {
        let mut cond: cnd_t = unsafe { core::mem::zeroed() };
        let _ = cnd_init(&mut cond as *mut cnd_t);

        let ret = cnd_signal(&mut cond as *mut cnd_t);
        assert_eq!(ret, 0, "cnd_signal 应返回 0");

        let ret = cnd_broadcast(&mut cond as *mut cnd_t);
        assert_eq!(ret, 0, "cnd_broadcast 应返回 0");

        cnd_destroy(&mut cond as *mut cnd_t);
    }
});

test!("test_cnd_timedwait_timeout" {
    // C11 timedwait 超时应返回 thrd_timedout
    {
        let mut cond: cnd_t = unsafe { core::mem::zeroed() };
        let mut mtx: mtx_t = unsafe { core::mem::zeroed() };
        let _ = cnd_init(&mut cond as *mut cnd_t);
        let _ = mtx_init(&mut mtx as *mut mtx_t, mtx_plain);

        // 需要持有互斥锁才能等待
        let _ = mtx_lock(&mut mtx as *mut mtx_t);

        let ts = timespec { tv_sec: 0, tv_nsec: 1000000 }; // 1ms
        let ret = cnd_timedwait(
            &mut cond as *mut cnd_t,
            &mut mtx as *mut mtx_t,
            &ts as *const timespec,
        );
        assert_eq!(ret, thrd_timedout, "cnd_timedwait 超时应返回 thrd_timedout");

        let _ = mtx_unlock(&mut mtx as *mut mtx_t);
        mtx_destroy(&mut mtx as *mut mtx_t);
        cnd_destroy(&mut cond as *mut cnd_t);
    }
});

// ============================================================================
// mtx_* 互斥锁测试
// ============================================================================

test!("test_mtx_init_plain" {
    // 初始化 mtx_plain 类型的互斥锁
    {
        let mut mtx: mtx_t = unsafe { core::mem::zeroed() };
        let ret = mtx_init(&mut mtx as *mut mtx_t, mtx_plain);
        assert_eq!(ret, 0, "mtx_init(plain) 应返回 0");

        mtx_destroy(&mut mtx as *mut mtx_t);
    }
});

test!("test_mtx_init_recursive" {
    // 初始化 mtx_recursive 类型的互斥锁
    {
        let mut mtx: mtx_t = unsafe { core::mem::zeroed() };
        let ret = mtx_init(&mut mtx as *mut mtx_t, mtx_recursive);
        assert_eq!(ret, 0, "mtx_init(recursive) 应返回 0");

        mtx_destroy(&mut mtx as *mut mtx_t);
    }
});

test!("test_mtx_lock_unlock" {
    // 基本 lock/trylock/unlock
    {
        let mut mtx: mtx_t = unsafe { core::mem::zeroed() };
        let _ = mtx_init(&mut mtx as *mut mtx_t, mtx_plain);

        let ret = mtx_lock(&mut mtx as *mut mtx_t);
        assert_eq!(ret, thrd_success, "mtx_lock 应返回 thrd_success");

        let ret = mtx_unlock(&mut mtx as *mut mtx_t);
        assert_eq!(ret, thrd_success, "mtx_unlock 应返回 thrd_success");

        mtx_destroy(&mut mtx as *mut mtx_t);
    }
});

test!("test_mtx_trylock" {
    // trylock 未锁定应成功, 锁定后应返回 thrd_busy
    {
        let mut mtx: mtx_t = unsafe { core::mem::zeroed() };
        let _ = mtx_init(&mut mtx as *mut mtx_t, mtx_plain);

        let ret = mtx_trylock(&mut mtx as *mut mtx_t);
        assert_eq!(ret, thrd_success, "mtx_trylock 未锁定应返回 thrd_success");

        let ret = mtx_trylock(&mut mtx as *mut mtx_t);
        assert_eq!(ret, thrd_busy, "mtx_trylock 已锁定应返回 thrd_busy");

        let _ = mtx_unlock(&mut mtx as *mut mtx_t);
        mtx_destroy(&mut mtx as *mut mtx_t);
    }
});

test!("test_mtx_timedlock_timeout" {
    // timedlock 超时应返回 thrd_timedout
    {
        let mut mtx: mtx_t = unsafe { core::mem::zeroed() };
        let _ = mtx_init(&mut mtx as *mut mtx_t, mtx_timed);

        let _ = mtx_lock(&mut mtx as *mut mtx_t);

        let ts = timespec { tv_sec: 0, tv_nsec: 0 };
        let ret = mtx_timedlock(&mut mtx as *mut mtx_t, &ts as *const timespec);
        assert_eq!(ret, thrd_timedout, "mtx_timedlock 超时应返回 thrd_timedout");

        let _ = mtx_unlock(&mut mtx as *mut mtx_t);
        mtx_destroy(&mut mtx as *mut mtx_t);
    }
});

// ============================================================================
// thrd_* 线程管理测试
// ============================================================================

test!("test_thrd_current_equal" {
    // thrd_current 和 thrd_equal 基本功能
    {
        let self_id = thrd_current();
        assert!(!self_id.is_null(), "thrd_current 应返回非空");

        assert_ne!(thrd_equal(self_id, self_id), 0, "thrd_equal 相同应返回非零");
        assert_eq!(
            thrd_equal(self_id, core::ptr::null_mut()),
            0,
            "thrd_equal 不同应返回 0"
        );
    }
});

test!("test_thrd_yield" {
    // thrd_yield 不应崩溃
    {
        thrd_yield();
        assert!(true, "thrd_yield 应正常返回");
    }
});

test!("test_thrd_create_join" {
    // 创建 C11 线程并 join
    {
        let mut thread: thrd_t = core::ptr::null_mut();
        let ret = thrd_create(
            &mut thread as *mut thrd_t,
            Some(c11_thread_func),
            core::ptr::null_mut(),
        );
        assert_eq!(ret, thrd_success, "thrd_create 应返回 thrd_success");

        let mut result: c_int = -1;
        let ret = thrd_join(thread, &mut result as *mut c_int);
        assert_eq!(ret, thrd_success, "thrd_join 应返回 thrd_success");
        assert_eq!(result, 0, "线程返回值应为 0");
    }
});

test!("test_thrd_create_return_value" {
    // C11 线程返回特定值
    {
        let mut thread: thrd_t = core::ptr::null_mut();
        let ret = thrd_create(
            &mut thread as *mut thrd_t,
            Some(c11_thread_func_with_value),
            core::ptr::null_mut(),
        );
        assert_eq!(ret, thrd_success);

        let mut result: c_int = -1;
        let ret = thrd_join(thread, &mut result as *mut c_int);
        assert_eq!(ret, thrd_success);
        assert_eq!(result, 42, "线程返回值应为 42");
    }
});

// ============================================================================
// tss_* 线程特定存储测试
// ============================================================================

test!("test_tss_create_delete" {
    // 创建/删除 TSS 键
    {
        let mut key: tss_t = 0;
        let ret = tss_create(&mut key as *mut tss_t, None);
        assert_eq!(ret, thrd_success, "tss_create 应返回 thrd_success");

        tss_delete(key);
        // tss_delete 返回 void
    }
});

test!("test_tss_set_get" {
    // 设置/获取 TSS 值
    {
        let mut key: tss_t = 0;
        let _ = tss_create(&mut key as *mut tss_t, None);

        let value: *mut c_void = 0xABCD as *mut c_void;
        let ret = tss_set(key, value);
        assert_eq!(ret, thrd_success, "tss_set 应返回 thrd_success");

        let result = tss_get(key);
        assert_eq!(result, value, "tss_get 应返回设置的值");

        tss_delete(key);
    }
});

test!("test_tss_get_unset" {
    // 未设置的键应返回 NULL
    {
        let mut key: tss_t = 0;
        let _ = tss_create(&mut key as *mut tss_t, None);

        let result = tss_get(key);
        assert!(result.is_null(), "未设置的 tss 键应返回 NULL");

        tss_delete(key);
    }
});
