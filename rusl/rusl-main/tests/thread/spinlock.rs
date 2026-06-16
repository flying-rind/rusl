//! 自旋锁 API 集成测试
//! 测试函数: pthread_spin_init, destroy, lock, trylock, unlock

use super::*;
use test_framework::test;

// ============================================================================
// pthread_spin 生命周期测试
// ============================================================================

test!("test_spin_init_destroy" {
    // 初始化/销毁自旋锁
    {
        let mut spin: pthread_spinlock_t = pthread_spinlock_t { __lock: 0 };
        let ret = pthread_spin_init(&mut spin as *mut pthread_spinlock_t, PTHREAD_PROCESS_PRIVATE);
        assert_eq!(ret, 0, "spin_init 应返回 0");

        let ret = pthread_spin_destroy(&mut spin as *mut pthread_spinlock_t);
        assert_eq!(ret, 0, "spin_destroy 应返回 0");
    }
});

test!("test_spin_init_pshared" {
    // 使用 PTHREAD_PROCESS_SHARED 初始化自旋锁
    {
        let mut spin: pthread_spinlock_t = pthread_spinlock_t { __lock: 0 };
        let ret = pthread_spin_init(&mut spin as *mut pthread_spinlock_t, PTHREAD_PROCESS_SHARED);
        assert_eq!(ret, 0, "spin_init(SHARED) 应返回 0");

        let _ = pthread_spin_destroy(&mut spin as *mut pthread_spinlock_t);
    }
});

// ============================================================================
// pthread_spin lock/trylock/unlock 测试
// ============================================================================

test!("test_spin_lock_unlock" {
    // 基本加锁/解锁
    {
        let mut spin: pthread_spinlock_t = pthread_spinlock_t { __lock: 0 };
        let _ = pthread_spin_init(&mut spin as *mut pthread_spinlock_t, PTHREAD_PROCESS_PRIVATE);

        let ret = pthread_spin_lock(&mut spin as *mut pthread_spinlock_t);
        assert_eq!(ret, 0, "spin_lock 应返回 0");

        let ret = pthread_spin_unlock(&mut spin as *mut pthread_spinlock_t);
        assert_eq!(ret, 0, "spin_unlock 应返回 0");

        let _ = pthread_spin_destroy(&mut spin as *mut pthread_spinlock_t);
    }
});

test!("test_spin_trylock_unlocked" {
    // trylock 未锁定的自旋锁应成功
    {
        let mut spin: pthread_spinlock_t = pthread_spinlock_t { __lock: 0 };
        let _ = pthread_spin_init(&mut spin as *mut pthread_spinlock_t, PTHREAD_PROCESS_PRIVATE);

        let ret = pthread_spin_trylock(&mut spin as *mut pthread_spinlock_t);
        assert_eq!(ret, 0, "spin_trylock 未锁定应返回 0");

        let ret = pthread_spin_unlock(&mut spin as *mut pthread_spinlock_t);
        assert_eq!(ret, 0, "spin_unlock 应返回 0");

        let _ = pthread_spin_destroy(&mut spin as *mut pthread_spinlock_t);
    }
});

test!("test_spin_trylock_locked" {
    // trylock 已锁定的自旋锁应返回 EBUSY
    {
        let mut spin: pthread_spinlock_t = pthread_spinlock_t { __lock: 0 };
        let _ = pthread_spin_init(&mut spin as *mut pthread_spinlock_t, PTHREAD_PROCESS_PRIVATE);

        let ret = pthread_spin_lock(&mut spin as *mut pthread_spinlock_t);
        assert_eq!(ret, 0, "spin_lock 应成功");

        let ret = pthread_spin_trylock(&mut spin as *mut pthread_spinlock_t);
        assert_eq!(ret, EBUSY, "spin_trylock 已锁定应返回 EBUSY");

        let _ = pthread_spin_unlock(&mut spin as *mut pthread_spinlock_t);
        let _ = pthread_spin_destroy(&mut spin as *mut pthread_spinlock_t);
    }
});

test!("test_spin_lock_repeated" {
    // 多次 lock/unlock 循环
    {
        let mut spin: pthread_spinlock_t = pthread_spinlock_t { __lock: 0 };
        let _ = pthread_spin_init(&mut spin as *mut pthread_spinlock_t, PTHREAD_PROCESS_PRIVATE);

        for _ in 0..3 {
            let ret = pthread_spin_lock(&mut spin as *mut pthread_spinlock_t);
            assert_eq!(ret, 0, "spin_lock 应成功");

            let ret = pthread_spin_unlock(&mut spin as *mut pthread_spinlock_t);
            assert_eq!(ret, 0, "spin_unlock 应成功");
        }

        let _ = pthread_spin_destroy(&mut spin as *mut pthread_spinlock_t);
    }
});
