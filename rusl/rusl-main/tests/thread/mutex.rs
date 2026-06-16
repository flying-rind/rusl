//! 互斥锁 API 集成测试
//! 测试函数: pthread_mutex_init, destroy, lock, trylock, unlock, mutexattr_*

use super::*;
use test_framework::test;

// 多线程测试的共享上下文 (通过 arg 传递)
#[repr(C)]
struct MutexTestCtx {
    mutex: *mut pthread_mutex_t,
    counter: AtomicI32,
}

extern "C" fn mutex_thread_func(arg: *mut c_void) -> *mut c_void {
    if arg.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        let ctx = &*(arg as *const MutexTestCtx);
        let ret = pthread_mutex_lock(ctx.mutex);
        if ret == 0 {
            let val = ctx.counter.load(Ordering::SeqCst);
            ctx.counter.store(val + 1, Ordering::SeqCst);
            pthread_mutex_unlock(ctx.mutex);
        }
    }
    core::ptr::null_mut()
}

// ============================================================================
// pthread_mutexattr 测试
// ============================================================================

test!("test_mutexattr_init_destroy" {
    // mutexattr init/destroy 基本功能
    {
        let mut attr: pthread_mutexattr_t = pthread_mutexattr_t { __attr: 0 };
        let ret = pthread_mutexattr_init(&mut attr as *mut pthread_mutexattr_t);
        assert_eq!(ret, 0, "mutexattr_init 应返回 0");

        let ret = pthread_mutexattr_destroy(&mut attr as *mut pthread_mutexattr_t);
        assert_eq!(ret, 0, "mutexattr_destroy 应返回 0");
    }
});

test!("test_mutexattr_set_get_type" {
    // 设置/获取互斥锁类型
    {
        let mut attr: pthread_mutexattr_t = pthread_mutexattr_t { __attr: 0 };
        let _ = pthread_mutexattr_init(&mut attr as *mut pthread_mutexattr_t);

        let ret = pthread_mutexattr_settype(&mut attr as *mut pthread_mutexattr_t, PTHREAD_MUTEX_RECURSIVE);
        assert_eq!(ret, 0, "settype(RECURSIVE) 应返回 0");

        let mut type_val: c_int = -1;
        let ret = pthread_mutexattr_gettype(
            &attr as *const pthread_mutexattr_t,
            &mut type_val as *mut c_int,
        );
        assert_eq!(ret, 0, "gettype 应返回 0");
        assert_eq!(type_val, PTHREAD_MUTEX_RECURSIVE, "type 应为 RECURSIVE");

        let _ = pthread_mutexattr_destroy(&mut attr as *mut pthread_mutexattr_t);
    }
});

test!("test_mutexattr_set_get_pshared" {
    // 设置/获取进程共享属性
    {
        let mut attr: pthread_mutexattr_t = pthread_mutexattr_t { __attr: 0 };
        let _ = pthread_mutexattr_init(&mut attr as *mut pthread_mutexattr_t);

        let ret = pthread_mutexattr_setpshared(&mut attr as *mut pthread_mutexattr_t, PTHREAD_PROCESS_SHARED);
        assert_eq!(ret, 0, "setpshared 应返回 0");

        let mut pshared: c_int = -1;
        let ret = pthread_mutexattr_getpshared(
            &attr as *const pthread_mutexattr_t,
            &mut pshared as *mut c_int,
        );
        assert_eq!(ret, 0, "getpshared 应返回 0");
        assert_eq!(pshared, PTHREAD_PROCESS_SHARED, "pshared 应为 PROCESS_SHARED");

        let _ = pthread_mutexattr_destroy(&mut attr as *mut pthread_mutexattr_t);
    }
});

test!("test_mutexattr_set_get_protocol" {
    // 设置/获取优先级协议
    {
        let mut attr: pthread_mutexattr_t = pthread_mutexattr_t { __attr: 0 };
        let _ = pthread_mutexattr_init(&mut attr as *mut pthread_mutexattr_t);

        let ret = pthread_mutexattr_setprotocol(&mut attr as *mut pthread_mutexattr_t, PTHREAD_PRIO_INHERIT);
        assert_eq!(ret, 0, "setprotocol 应返回 0");

        let mut protocol: c_int = -1;
        let ret = pthread_mutexattr_getprotocol(
            &attr as *const pthread_mutexattr_t,
            &mut protocol as *mut c_int,
        );
        assert_eq!(ret, 0, "getprotocol 应返回 0");
        assert_eq!(protocol, PTHREAD_PRIO_INHERIT, "protocol 应为 PRIO_INHERIT");

        let _ = pthread_mutexattr_destroy(&mut attr as *mut pthread_mutexattr_t);
    }
});

test!("test_mutexattr_set_get_robust" {
    // 设置/获取健壮性
    {
        let mut attr: pthread_mutexattr_t = pthread_mutexattr_t { __attr: 0 };
        let _ = pthread_mutexattr_init(&mut attr as *mut pthread_mutexattr_t);

        let ret = pthread_mutexattr_setrobust(&mut attr as *mut pthread_mutexattr_t, PTHREAD_MUTEX_ROBUST);
        assert_eq!(ret, 0, "setrobust 应返回 0");

        let mut robust: c_int = -1;
        let ret = pthread_mutexattr_getrobust(
            &attr as *const pthread_mutexattr_t,
            &mut robust as *mut c_int,
        );
        assert_eq!(ret, 0, "getrobust 应返回 0");
        assert_eq!(robust, PTHREAD_MUTEX_ROBUST, "robust 应为 ROBUST");

        let _ = pthread_mutexattr_destroy(&mut attr as *mut pthread_mutexattr_t);
    }
});

// ============================================================================
// pthread_mutex 生命周期测试
// ============================================================================

test!("test_mutex_init_default" {
    // 使用 NULL 属性初始化互斥锁应返回 0
    {
        let mut mutex: pthread_mutex_t = unsafe { core::mem::zeroed() };
        let ret = pthread_mutex_init(&mut mutex as *mut pthread_mutex_t, core::ptr::null());
        assert_eq!(ret, 0, "mutex_init(NULL attr) 应返回 0");

        let ret = pthread_mutex_destroy(&mut mutex as *mut pthread_mutex_t);
        assert_eq!(ret, 0, "mutex_destroy 应返回 0");
    }
});

test!("test_mutex_init_with_attr" {
    // 使用属性初始化互斥锁
    {
        let mut attr: pthread_mutexattr_t = pthread_mutexattr_t { __attr: 0 };
        let _ = pthread_mutexattr_init(&mut attr as *mut pthread_mutexattr_t);
        let _ = pthread_mutexattr_settype(&mut attr as *mut pthread_mutexattr_t, PTHREAD_MUTEX_RECURSIVE);

        let mut mutex: pthread_mutex_t = unsafe { core::mem::zeroed() };
        let ret = pthread_mutex_init(&mut mutex as *mut pthread_mutex_t, &attr as *const pthread_mutexattr_t);
        assert_eq!(ret, 0, "mutex_init(with attr) 应返回 0");

        let ret = pthread_mutex_destroy(&mut mutex as *mut pthread_mutex_t);
        assert_eq!(ret, 0, "mutex_destroy 应返回 0");

        let _ = pthread_mutexattr_destroy(&mut attr as *mut pthread_mutexattr_t);
    }
});

// ============================================================================
// pthread_mutex lock/trylock/unlock 测试
// ============================================================================

test!("test_mutex_lock_unlock" {
    // 基本锁/解锁操作
    {
        let mut mutex: pthread_mutex_t = unsafe { core::mem::zeroed() };
        let _ = pthread_mutex_init(&mut mutex as *mut pthread_mutex_t, core::ptr::null());

        let ret = pthread_mutex_lock(&mut mutex as *mut pthread_mutex_t);
        assert_eq!(ret, 0, "lock 应返回 0");

        let ret = pthread_mutex_unlock(&mut mutex as *mut pthread_mutex_t);
        assert_eq!(ret, 0, "unlock 应返回 0");

        let _ = pthread_mutex_destroy(&mut mutex as *mut pthread_mutex_t);
    }
});

test!("test_mutex_trylock_unlocked" {
    // trylock 未锁定的互斥锁应成功
    {
        let mut mutex: pthread_mutex_t = unsafe { core::mem::zeroed() };
        let _ = pthread_mutex_init(&mut mutex as *mut pthread_mutex_t, core::ptr::null());

        let ret = pthread_mutex_trylock(&mut mutex as *mut pthread_mutex_t);
        assert_eq!(ret, 0, "trylock 未锁定应返回 0");

        let ret = pthread_mutex_unlock(&mut mutex as *mut pthread_mutex_t);
        assert_eq!(ret, 0, "unlock 应返回 0");

        let _ = pthread_mutex_destroy(&mut mutex as *mut pthread_mutex_t);
    }
});

test!("test_mutex_trylock_locked" {
    // trylock 已锁定的互斥锁应返回 EBUSY
    {
        let mut mutex: pthread_mutex_t = unsafe { core::mem::zeroed() };
        let _ = pthread_mutex_init(&mut mutex as *mut pthread_mutex_t, core::ptr::null());

        let ret = pthread_mutex_lock(&mut mutex as *mut pthread_mutex_t);
        assert_eq!(ret, 0, "lock 应成功");

        let ret = pthread_mutex_trylock(&mut mutex as *mut pthread_mutex_t);
        assert_eq!(ret, EBUSY, "trylock 已锁定应返回 EBUSY");

        let ret = pthread_mutex_unlock(&mut mutex as *mut pthread_mutex_t);
        assert_eq!(ret, 0, "unlock 应成功");

        let _ = pthread_mutex_destroy(&mut mutex as *mut pthread_mutex_t);
    }
});

test!("test_mutex_recursive_lock" {
    // 递归互斥锁允许多次加锁
    {
        let mut attr: pthread_mutexattr_t = pthread_mutexattr_t { __attr: 0 };
        let _ = pthread_mutexattr_init(&mut attr as *mut pthread_mutexattr_t);
        let _ = pthread_mutexattr_settype(&mut attr as *mut pthread_mutexattr_t, PTHREAD_MUTEX_RECURSIVE);

        let mut mutex: pthread_mutex_t = unsafe { core::mem::zeroed() };
        let _ = pthread_mutex_init(&mut mutex as *mut pthread_mutex_t, &attr as *const pthread_mutexattr_t);

        let ret = pthread_mutex_lock(&mut mutex as *mut pthread_mutex_t);
        assert_eq!(ret, 0, "第一次 lock 应成功");

        let ret = pthread_mutex_lock(&mut mutex as *mut pthread_mutex_t);
        assert_eq!(ret, 0, "递归 lock 应成功");

        let ret = pthread_mutex_unlock(&mut mutex as *mut pthread_mutex_t);
        assert_eq!(ret, 0, "第一次 unlock 应成功");

        let ret = pthread_mutex_unlock(&mut mutex as *mut pthread_mutex_t);
        assert_eq!(ret, 0, "第二次 unlock 应成功");

        let _ = pthread_mutex_destroy(&mut mutex as *mut pthread_mutex_t);
        let _ = pthread_mutexattr_destroy(&mut attr as *mut pthread_mutexattr_t);
    }
});

// ============================================================================
// pthread_mutex_timedlock 测试
// ============================================================================

test!("test_mutex_timedlock_timeout" {
    // timedlock 超时, 应返回 ETIMEDOUT
    {
        let mut mutex: pthread_mutex_t = unsafe { core::mem::zeroed() };
        let _ = pthread_mutex_init(&mut mutex as *mut pthread_mutex_t, core::ptr::null());

        let _ = pthread_mutex_lock(&mut mutex as *mut pthread_mutex_t);

        let ts = timespec { tv_sec: 0, tv_nsec: 0 };
        let ret = pthread_mutex_timedlock(&mut mutex as *mut pthread_mutex_t, &ts as *const timespec);
        assert_eq!(ret, ETIMEDOUT, "timedlock 已锁定且超时应返回 ETIMEDOUT");

        let _ = pthread_mutex_unlock(&mut mutex as *mut pthread_mutex_t);
        let _ = pthread_mutex_destroy(&mut mutex as *mut pthread_mutex_t);
    }
});

// ============================================================================
// pthread_mutex 多线程竞争测试
// ============================================================================

test!("test_mutex_multithread" {
    // 多线程通过互斥锁保护共享计数器 (使用栈分配 + arg 传递)
    {
        let mut mutex: pthread_mutex_t = unsafe { core::mem::zeroed() };
        let ret = pthread_mutex_init(&mut mutex as *mut pthread_mutex_t, core::ptr::null());
        assert_eq!(ret, 0, "init mutex 应成功");

        let ctx = MutexTestCtx {
            mutex: &mut mutex as *mut pthread_mutex_t,
            counter: AtomicI32::new(0),
        };

        const N: usize = 2;
        let mut threads: [pthread_t; N] = [core::ptr::null_mut(); N];

        for i in 0..N {
            let ret = pthread_create(
                &mut threads[i] as *mut pthread_t,
                core::ptr::null(),
                Some(mutex_thread_func),
                &raw const ctx as *mut c_void,
            );
            assert_eq!(ret, 0);
        }

        for i in 0..N {
            let mut result: *mut c_void = core::ptr::null_mut();
            let ret = pthread_join(threads[i], &mut result as *mut *mut c_void);
            assert_eq!(ret, 0);
        }

        assert_eq!(ctx.counter.load(Ordering::SeqCst), N as i32, "计数器应为 N");

        let ret = pthread_mutex_destroy(&mut mutex as *mut pthread_mutex_t);
        assert_eq!(ret, 0, "destroy mutex 应成功");
    }
});

// ============================================================================
// pthread_mutex_consistent 测试
// ============================================================================

test!("test_mutex_consistent" {
    // 对正常互斥锁调用 consistent 应返回 EINVAL
    {
        let mut mutex: pthread_mutex_t = unsafe { core::mem::zeroed() };
        let _ = pthread_mutex_init(&mut mutex as *mut pthread_mutex_t, core::ptr::null());

        let ret = pthread_mutex_consistent(&mut mutex as *mut pthread_mutex_t);
        assert_eq!(ret, EINVAL, "consistent 对正常 mutex 应返回 EINVAL");

        let _ = pthread_mutex_destroy(&mut mutex as *mut pthread_mutex_t);
    }
});

// ============================================================================
// pthread_mutex_getprioceiling / setprioceiling 测试
// ============================================================================

test!("test_mutex_getprioceiling" {
    // getprioceiling 应返回 EINVAL
    {
        let mut mutex: pthread_mutex_t = unsafe { core::mem::zeroed() };
        let _ = pthread_mutex_init(&mut mutex as *mut pthread_mutex_t, core::ptr::null());

        let mut ceiling: c_int = -1;
        let ret = pthread_mutex_getprioceiling(
            &mutex as *const pthread_mutex_t,
            &mut ceiling as *mut c_int,
        );
        assert_eq!(ret, EINVAL, "getprioceiling 应返回 EINVAL");

        let _ = pthread_mutex_destroy(&mut mutex as *mut pthread_mutex_t);
    }
});

test!("test_mutex_setprioceiling" {
    // setprioceiling 应返回 EINVAL
    {
        let mut mutex: pthread_mutex_t = unsafe { core::mem::zeroed() };
        let _ = pthread_mutex_init(&mut mutex as *mut pthread_mutex_t, core::ptr::null());

        let mut old: c_int = -1;
        let ret = pthread_mutex_setprioceiling(
            &mut mutex as *mut pthread_mutex_t,
            10,
            &mut old as *mut c_int,
        );
        assert_eq!(ret, EINVAL, "setprioceiling 应返回 EINVAL");

        let _ = pthread_mutex_destroy(&mut mutex as *mut pthread_mutex_t);
    }
});
