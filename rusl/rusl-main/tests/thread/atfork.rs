//! Fork 处理 API 集成测试
//! 测试函数: pthread_atfork

use super::*;
use test_framework::test;

// 用于回调的原子计数器
static PREPARE_CNT: AtomicI32 = AtomicI32::new(0);
static PARENT_CNT: AtomicI32 = AtomicI32::new(0);
static CHILD_CNT: AtomicI32 = AtomicI32::new(0);

extern "C" fn prepare_handler() {
    PREPARE_CNT.fetch_add(1, Ordering::SeqCst);
}

extern "C" fn parent_handler() {
    PARENT_CNT.fetch_add(1, Ordering::SeqCst);
}

extern "C" fn child_handler() {
    CHILD_CNT.fetch_add(1, Ordering::SeqCst);
}

// ============================================================================
// pthread_atfork 测试
// ============================================================================

test!("test_pthread_atfork_all_null" {
    // 所有回调为 NULL 时应返回 0
    {
        let ret = pthread_atfork(None, None, None);
        assert_eq!(ret, 0, "pthread_atfork(NULL, NULL, NULL) 应返回 0");
    }
});

test!("test_pthread_atfork_prepare_only" {
    // 只注册 prepare 回调
    {
        let ret = pthread_atfork(Some(prepare_handler), None, None);
        assert_eq!(ret, 0, "pthread_atfork(prepare) 应返回 0");
    }
});

test!("test_pthread_atfork_parent_only" {
    // 只注册 parent 回调
    {
        let ret = pthread_atfork(None, Some(parent_handler), None);
        assert_eq!(ret, 0, "pthread_atfork(parent) 应返回 0");
    }
});

test!("test_pthread_atfork_child_only" {
    // 只注册 child 回调
    {
        let ret = pthread_atfork(None, None, Some(child_handler));
        assert_eq!(ret, 0, "pthread_atfork(child) 应返回 0");
    }
});

test!("test_pthread_atfork_all" {
    // 注册所有三个回调
    {
        let ret = pthread_atfork(
            Some(prepare_handler),
            Some(parent_handler),
            Some(child_handler),
        );
        assert_eq!(ret, 0, "pthread_atfork(all) 应返回 0");
    }
});
