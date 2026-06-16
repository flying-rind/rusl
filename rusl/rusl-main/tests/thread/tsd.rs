//! 线程局部存储 (TSD) API 集成测试
//! 测试函数: pthread_key_create, pthread_key_delete, pthread_getspecific, pthread_setspecific

use super::*;
use test_framework::test;

// ============================================================================
// pthread_key_create 测试
// ============================================================================

test!("test_key_create_default" {
    // 创建 TSD 键 (无析构函数)
    {
        let mut key: pthread_key_t = 0;
        let ret = pthread_key_create(&mut key as *mut pthread_key_t, None);
        assert_eq!(ret, 0, "key_create 应返回 0");

        let ret = pthread_key_delete(key);
        assert_eq!(ret, 0, "key_delete 应返回 0");
    }
});


// ============================================================================
// pthread_getspecific / pthread_setspecific 测试
// ============================================================================

test!("test_setspecific_getspecific" {
    // 设置和获取 TSD 值
    {
        let mut key: pthread_key_t = 0;
        let _ = pthread_key_create(&mut key as *mut pthread_key_t, None);

        let value: *mut c_void = 0x1234 as *mut c_void;
        let ret = pthread_setspecific(key, value as *const c_void);
        assert_eq!(ret, 0, "setspecific 应返回 0");

        let result = pthread_getspecific(key);
        assert_eq!(result, value, "getspecific 应返回设置的值");

        let _ = pthread_key_delete(key);
    }
});

test!("test_getspecific_unset_key" {
    // 未设置的键应返回 NULL
    {
        let mut key: pthread_key_t = 0;
        let _ = pthread_key_create(&mut key as *mut pthread_key_t, None);

        let result = pthread_getspecific(key);
        assert!(result.is_null(), "未设置的键应返回 NULL");

        let _ = pthread_key_delete(key);
    }
});

test!("test_setspecific_null" {
    // 设置值为 NULL
    {
        let mut key: pthread_key_t = 0;
        let _ = pthread_key_create(&mut key as *mut pthread_key_t, None);

        let ret = pthread_setspecific(key, core::ptr::null());
        assert_eq!(ret, 0, "setspecific(NULL) 应返回 0");

        let result = pthread_getspecific(key);
        assert!(result.is_null(), "getspecific 应返回 NULL");

        let _ = pthread_key_delete(key);
    }
});

test!("test_setspecific_overwrite" {
    // 覆盖 TSD 值
    {
        let mut key: pthread_key_t = 0;
        let _ = pthread_key_create(&mut key as *mut pthread_key_t, None);

        let value1: *mut c_void = 0xAAAA as *mut c_void;
        let value2: *mut c_void = 0xBBBB as *mut c_void;

        let ret = pthread_setspecific(key, value1 as *const c_void);
        assert_eq!(ret, 0);
        assert_eq!(pthread_getspecific(key), value1);

        let ret = pthread_setspecific(key, value2 as *const c_void);
        assert_eq!(ret, 0);
        assert_eq!(pthread_getspecific(key), value2);

        let _ = pthread_key_delete(key);
    }
});


// ============================================================================
// TSD 多键测试
// ============================================================================

test!("test_multiple_keys" {
    // 创建多个键, 各自独立存储
    {
        let mut key1: pthread_key_t = 0;
        let mut key2: pthread_key_t = 0;
        let _ = pthread_key_create(&mut key1 as *mut pthread_key_t, None);
        let _ = pthread_key_create(&mut key2 as *mut pthread_key_t, None);

        let val1: *mut c_void = 0x1111 as *mut c_void;
        let val2: *mut c_void = 0x2222 as *mut c_void;

        let _ = pthread_setspecific(key1, val1 as *const c_void);
        let _ = pthread_setspecific(key2, val2 as *const c_void);

        assert_eq!(pthread_getspecific(key1), val1, "key1 值应独立");
        assert_eq!(pthread_getspecific(key2), val2, "key2 值应独立");

        let _ = pthread_key_delete(key1);
        let _ = pthread_key_delete(key2);
    }
});

// ============================================================================
// TSD 跨线程测试
// ============================================================================

test!("test_tsd_across_threads" {
    // TSD 值在不同线程中独立
    {
        static TSD_KEY: AtomicI32 = AtomicI32::new(0);
        static CHILD_SET: AtomicBool = AtomicBool::new(false);

        extern "C" fn tsd_child_func(_arg: *mut c_void) -> *mut c_void {
            let key = TSD_KEY.load(Ordering::SeqCst) as pthread_key_t;
            // 在子线程中, 键应该还未被设置 (检查并记录结果)
            let val = pthread_getspecific(key);
            if !val.is_null() {
                // 子线程获取了非期望值
                CHILD_SET.store(false, Ordering::SeqCst);
            } else {
                // 设置子线程的值
                let child_val: *mut c_void = 0xDEAD as *mut c_void;
                let _ = pthread_setspecific(key, child_val as *const c_void);
                CHILD_SET.store(true, Ordering::SeqCst);
            }
            core::ptr::null_mut()
        }

        {
            let mut key: pthread_key_t = 0;
            let _ = pthread_key_create(&mut key as *mut pthread_key_t, None);
            TSD_KEY.store(key as i32, Ordering::SeqCst);

            // 主线程设置值
            let parent_val: *mut c_void = 0xBEEF as *mut c_void;
            let _ = pthread_setspecific(key, parent_val as *const c_void);

            // 创建子线程
            let mut child: pthread_t = core::ptr::null_mut();
            let ret = pthread_create(
                &mut child as *mut pthread_t,
                core::ptr::null(),
                Some(tsd_child_func),
                core::ptr::null_mut(),
            );
            assert_eq!(ret, 0);

            let mut result: *mut c_void = core::ptr::null_mut();
            let _ = pthread_join(child, &mut result as *mut *mut c_void);

            // 主线程的值不变
            assert_eq!(pthread_getspecific(key), parent_val, "父线程 TSD 值不变");

            assert!(CHILD_SET.load(Ordering::SeqCst), "子线程应设置了 TSD 值");

            let _ = pthread_key_delete(key);
        }
    }
});
