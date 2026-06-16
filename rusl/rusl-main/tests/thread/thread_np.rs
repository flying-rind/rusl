//! GNU 扩展线程属性 API 集成测试
//! 测试函数: pthread_getattr_np, pthread_getattr_default_np, pthread_setattr_default_np

use super::*;
use test_framework::test;

// ============================================================================
// pthread_getattr_np 测试
// ============================================================================

test!("test_pthread_getattr_np_self" {
    // 获取当前线程的属性, 应返回 0
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let self_id = pthread_self();
        let ret = pthread_getattr_np(self_id, &mut attr as *mut pthread_attr_t);
        assert_eq!(ret, 0, "pthread_getattr_np 应返回 0");

        // 可以读取 detachstate
        let mut state: c_int = -1;
        let ret = pthread_attr_getdetachstate(
            &attr as *const pthread_attr_t,
            &mut state as *mut c_int,
        );
        assert_eq!(ret, 0, "getdetachstate 应成功");

        // 可以读取 stacksize
        let mut stacksize: usize = 0;
        let ret = pthread_attr_getstacksize(
            &attr as *const pthread_attr_t,
            &mut stacksize as *mut usize,
        );
        assert_eq!(ret, 0, "getstacksize 应成功");
        assert!(stacksize > 0, "stacksize 应为正数");
    }
});


// ============================================================================
// pthread_getattr_default_np / pthread_setattr_default_np 测试
// ============================================================================

test!("test_pthread_getattr_default_np" {
    // 获取默认线程属性
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let ret = pthread_getattr_default_np(&mut attr as *mut pthread_attr_t);
        assert_eq!(ret, 0, "pthread_getattr_default_np 应返回 0");

        // 默认属性应有有效的 detachstate
        let mut state: c_int = -1;
        let ret = pthread_attr_getdetachstate(
            &attr as *const pthread_attr_t,
            &mut state as *mut c_int,
        );
        assert_eq!(ret, 0, "getdetachstate 应成功");
        assert_eq!(state, PTHREAD_CREATE_JOINABLE, "默认 detachstate 应为 JOINABLE");

        // 默认属性应有有效的 stacksize
        let mut stacksize: usize = 0;
        let ret = pthread_attr_getstacksize(
            &attr as *const pthread_attr_t,
            &mut stacksize as *mut usize,
        );
        assert_eq!(ret, 0, "getstacksize 应成功");
        assert!(stacksize > 0, "默认 stacksize 应为正数");
    }
});

test!("test_pthread_setattr_default_np" {
    // 设置默认线程属性 (musl 可能返回 EINVAL)
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let ret = pthread_attr_init(&mut attr as *mut pthread_attr_t);
        assert_eq!(ret, 0, "init 应成功");

        let ret = pthread_setattr_default_np(&attr as *const pthread_attr_t);
        // musl 可能不支持设置默认属性, 返回 0 或 EINVAL(22)
        assert!(
            ret == 0 || ret == 22, // 22 = EINVAL
            "pthread_setattr_default_np 应返回 0 或 EINVAL, got {}",
            ret
        );
    }
});
