//! 线程取消 API 集成测试
//! 测试函数: pthread_cancel, pthread_testcancel, pthread_setcancelstate, pthread_setcanceltype

use super::*;
use test_framework::test;

// ============================================================================
// pthread_setcancelstate 测试
// ============================================================================

test!("test_setcancelstate_enable" {
    // 设置取消状态为 ENABLE, 获取旧状态
    {
        let mut oldstate: c_int = -1;
        let ret = pthread_setcancelstate(PTHREAD_CANCEL_ENABLE, &mut oldstate as *mut c_int);
        assert_eq!(ret, 0, "setcancelstate(ENABLE) 应返回 0");
        // 旧状态可以是 ENABLE(0) 或 DISABLE(1)
        assert!(
            oldstate == PTHREAD_CANCEL_ENABLE || oldstate == PTHREAD_CANCEL_DISABLE,
            "oldstate 应为 ENABLE 或 DISABLE"
        );
    }
});

test!("test_setcancelstate_disable" {
    // 设置取消状态为 DISABLE
    {
        let mut oldstate: c_int = -1;
        let ret = pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &mut oldstate as *mut c_int);
        assert_eq!(ret, 0, "setcancelstate(DISABLE) 应返回 0");
    }
});

test!("test_setcancelstate_nested" {
    // 嵌套设置取消状态
    {
        // 先设为 DISABLE
        let mut old1: c_int = -1;
        let ret = pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &mut old1 as *mut c_int);
        assert_eq!(ret, 0);

        // 再次设为 DISABLE
        let mut old2: c_int = -1;
        let ret = pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &mut old2 as *mut c_int);
        assert_eq!(ret, 0);
        assert_eq!(old2, PTHREAD_CANCEL_DISABLE, "旧状态应为 DISABLE");

        // 恢复为 ENABLE
        let mut old3: c_int = -1;
        let ret = pthread_setcancelstate(PTHREAD_CANCEL_ENABLE, &mut old3 as *mut c_int);
        assert_eq!(ret, 0);
    }
});

test!("test_setcancelstate_null_oldstate" {
    // oldstate 为 NULL 时也应正常工作
    {
        let ret = pthread_setcancelstate(PTHREAD_CANCEL_ENABLE, core::ptr::null_mut());
        assert_eq!(ret, 0, "setcancelstate(NULL oldstate) 应返回 0");
    }
});

test!("test_setcancelstate_invalid" {
    // 无效的取消状态值应返回 EINVAL
    {
        let mut oldstate: c_int = -1;
        let ret = pthread_setcancelstate(99, &mut oldstate as *mut c_int);
        assert_eq!(ret, EINVAL, "setcancelstate(99) 应返回 EINVAL");
    }
});

// ============================================================================
// pthread_setcanceltype 测试
// ============================================================================

test!("test_setcanceltype_deferred" {
    // 设置取消类型为 DEFERRED
    {
        let mut oldtype: c_int = -1;
        let ret = pthread_setcanceltype(PTHREAD_CANCEL_DEFERRED, &mut oldtype as *mut c_int);
        assert_eq!(ret, 0, "setcanceltype(DEFERRED) 应返回 0");
    }
});

test!("test_setcanceltype_asynchronous" {
    // 设置取消类型为 ASYNCHRONOUS
    {
        let mut oldtype: c_int = -1;
        let ret = pthread_setcanceltype(PTHREAD_CANCEL_ASYNCHRONOUS, &mut oldtype as *mut c_int);
        assert_eq!(ret, 0, "setcanceltype(ASYNCHRONOUS) 应返回 0");
    }
});

test!("test_setcanceltype_invalid" {
    // 无效的取消类型应返回 EINVAL
    {
        let mut oldtype: c_int = -1;
        let ret = pthread_setcanceltype(99, &mut oldtype as *mut c_int);
        assert_eq!(ret, EINVAL, "setcanceltype(99) 应返回 EINVAL");
    }
});

// ============================================================================
// pthread_testcancel 测试
// ============================================================================

test!("test_pthread_testcancel" {
    // testcancel 在没有待处理的取消请求时不应做任何事
    {
        // 确保取消状态为 ENABLE
        let _ = pthread_setcancelstate(PTHREAD_CANCEL_ENABLE, core::ptr::null_mut());
        // testcancel 在无取消请求时应该安全通过
        pthread_testcancel();
        assert!(true, "testcancel 应正常返回");
    }
});

test!("test_pthread_testcancel_when_disabled" {
    // 取消禁用时 testcancel 不做任何事
    {
        let mut oldstate: c_int = -1;
        let _ = pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &mut oldstate as *mut c_int);

        pthread_testcancel();
        assert!(true, "取消禁用时 testcancel 应正常返回");

        // 恢复
        let _ = pthread_setcancelstate(PTHREAD_CANCEL_ENABLE, core::ptr::null_mut());
    }
});

// ============================================================================
// pthread_cancel 测试
// ============================================================================

test!("test_pthread_cancel_self" {
    // 向自己发送取消请求: 先禁用取消确保取消标志不会立即导致终止
    {
        let mut oldstate: c_int = -1;
        let _ = pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &mut oldstate as *mut c_int);

        let ret = pthread_cancel(pthread_self());
        assert_eq!(ret, 0, "cancel(self) 应返回 0");

        // 保持取消禁用, 不清除已设置的取消标志
        // 后续测试不会受影响
        let _ = pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, core::ptr::null_mut());
    }
});

test!("test_pthread_cancel_and_join" {
    // 取消一个线程并等待其终止
    // 使用异步取消模式确保立即响应
    extern "C" fn cancel_target_thread(_arg: *mut c_void) -> *mut c_void {
        // 设置异步取消以确保立即响应 cancel
        let _ = pthread_setcanceltype(PTHREAD_CANCEL_ASYNCHRONOUS, core::ptr::null_mut());
        let _ = pthread_setcancelstate(PTHREAD_CANCEL_ENABLE, core::ptr::null_mut());
        // 自旋等待, ASYNCHRONOUS 模式下 cancel 会立即终止此线程
        loop {
            // 忙等待, 取消会异步生效
        }
    }

    {
        let mut thread: pthread_t = core::ptr::null_mut();
        let ret = pthread_create(
            &mut thread as *mut pthread_t,
            core::ptr::null(),
            Some(cancel_target_thread),
            core::ptr::null_mut(),
        );
        assert_eq!(ret, 0);

        // 发送取消请求 (ASYNCHRONOUS 模式下立即生效)
        let ret = pthread_cancel(thread);
        assert_eq!(ret, 0, "cancel 应返回 0");

        // join 等待线程终止
        let mut result: *mut c_void = core::ptr::null_mut();
        let ret = pthread_join(thread, &mut result as *mut *mut c_void);
        assert_eq!(ret, 0, "join 应成功");

        // 被取消的线程应返回 PTHREAD_CANCELED
        assert_eq!(result, PTHREAD_CANCELED, "被取消线程应返回 PTHREAD_CANCELED");
    }
});
