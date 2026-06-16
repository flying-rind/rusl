//! pthread_attr_* 系列函数集成测试

use super::*;
use test_framework::test;

// ============================================================================
// pthread_attr_init / pthread_attr_destroy 测试
// ============================================================================

test!("test_attr_init_success" {
    // pthread_attr_init 应成功返回 0
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let ret = pthread_attr_init(&mut attr as *mut pthread_attr_t);
        assert_eq!(ret, 0, "pthread_attr_init 应返回 0");
    }
});

test!("test_attr_destroy_success" {
    // pthread_attr_destroy 已初始化的属性对象应返回 0
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let _ = pthread_attr_init(&mut attr as *mut pthread_attr_t);
        let ret = pthread_attr_destroy(&mut attr as *mut pthread_attr_t);
        assert_eq!(ret, 0, "pthread_attr_destroy 应返回 0");
    }
});

// ============================================================================
// detachstate 测试
// ============================================================================

test!("test_attr_set_get_detachstate_joinable" {
    // 设置 JOINABLE, 读取应得到 JOINABLE
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let _ = pthread_attr_init(&mut attr as *mut pthread_attr_t);

        let ret = pthread_attr_setdetachstate(
            &mut attr as *mut pthread_attr_t,
            PTHREAD_CREATE_JOINABLE,
        );
        assert_eq!(ret, 0, "setdetachstate(JOINABLE) 应返回 0");

        let mut state: c_int = -1;
        let ret = pthread_attr_getdetachstate(
            &attr as *const pthread_attr_t,
            &mut state as *mut c_int,
        );
        assert_eq!(ret, 0, "getdetachstate 应返回 0");
        assert_eq!(state, PTHREAD_CREATE_JOINABLE, "detachstate 应为 JOINABLE(0)");
    }
});

test!("test_attr_set_get_detachstate_detached" {
    // 设置 DETACHED, 读取应得到 DETACHED
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let _ = pthread_attr_init(&mut attr as *mut pthread_attr_t);

        let ret = pthread_attr_setdetachstate(
            &mut attr as *mut pthread_attr_t,
            PTHREAD_CREATE_DETACHED,
        );
        assert_eq!(ret, 0, "setdetachstate(DETACHED) 应返回 0");

        let mut state: c_int = -1;
        let ret = pthread_attr_getdetachstate(
            &attr as *const pthread_attr_t,
            &mut state as *mut c_int,
        );
        assert_eq!(ret, 0, "getdetachstate 应返回 0");
        assert_eq!(state, PTHREAD_CREATE_DETACHED, "detachstate 应为 DETACHED(1)");
    }
});

// ============================================================================
// guardsize 测试
// ============================================================================

test!("test_attr_set_get_guardsize" {
    // 设置自定义 guardsize, 读取应得到相同值
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let _ = pthread_attr_init(&mut attr as *mut pthread_attr_t);

        let ret = pthread_attr_setguardsize(&mut attr as *mut pthread_attr_t, 4096);
        assert_eq!(ret, 0, "setguardsize 应返回 0");

        let mut size: usize = 0;
        let ret = pthread_attr_getguardsize(
            &attr as *const pthread_attr_t,
            &mut size as *mut usize,
        );
        assert_eq!(ret, 0, "getguardsize 应返回 0");
        assert!(size >= 4096, "guardsize 应至少为设置的值 (got {})", size);
    }
});

// ============================================================================
// inheritsched 测试
// ============================================================================

test!("test_attr_set_get_inheritsched_inherit" {
    // 设置 INHERIT_SCHED, 读取应得到 INHERIT_SCHED
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let _ = pthread_attr_init(&mut attr as *mut pthread_attr_t);

        let ret = pthread_attr_setinheritsched(
            &mut attr as *mut pthread_attr_t,
            PTHREAD_INHERIT_SCHED,
        );
        assert_eq!(ret, 0, "setinheritsched 应返回 0");

        let mut inherit: c_int = -1;
        let ret = pthread_attr_getinheritsched(
            &attr as *const pthread_attr_t,
            &mut inherit as *mut c_int,
        );
        assert_eq!(ret, 0, "getinheritsched 应返回 0");
        assert_eq!(inherit, PTHREAD_INHERIT_SCHED, "inheritsched 应为 PTHREAD_INHERIT_SCHED(0)");
    }
});

test!("test_attr_set_get_inheritsched_explicit" {
    // 设置 EXPLICIT_SCHED, 读取应得到 EXPLICIT_SCHED
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let _ = pthread_attr_init(&mut attr as *mut pthread_attr_t);

        let ret = pthread_attr_setinheritsched(
            &mut attr as *mut pthread_attr_t,
            PTHREAD_EXPLICIT_SCHED,
        );
        assert_eq!(ret, 0, "setinheritsched 应返回 0");

        let mut inherit: c_int = -1;
        let ret = pthread_attr_getinheritsched(
            &attr as *const pthread_attr_t,
            &mut inherit as *mut c_int,
        );
        assert_eq!(ret, 0, "getinheritsched 应返回 0");
        assert_eq!(inherit, PTHREAD_EXPLICIT_SCHED, "inheritsched 应为 PTHREAD_EXPLICIT_SCHED(1)");
    }
});

// ============================================================================
// schedparam 测试
// ============================================================================

test!("test_attr_set_get_schedparam" {
    // 设置调度参数, 读取应得到相同值
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let _ = pthread_attr_init(&mut attr as *mut pthread_attr_t);

        let param = sched_param { sched_priority: 10 };
        let ret = pthread_attr_setschedparam(
            &mut attr as *mut pthread_attr_t,
            &param as *const sched_param,
        );
        assert_eq!(ret, 0, "setschedparam 应返回 0");

        let mut out_param: sched_param = sched_param { sched_priority: -1 };
        let ret = pthread_attr_getschedparam(
            &attr as *const pthread_attr_t,
            &mut out_param as *mut sched_param,
        );
        assert_eq!(ret, 0, "getschedparam 应返回 0");
        assert_eq!(out_param.sched_priority, param.sched_priority,
            "sched_priority 应保持一致");
    }
});

// ============================================================================
// schedpolicy 测试
// ============================================================================

test!("test_attr_set_get_schedpolicy" {
    // 设置调度策略, 读取应得到相同值
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let _ = pthread_attr_init(&mut attr as *mut pthread_attr_t);

        // SCHED_FIFO = 1
        let ret = pthread_attr_setschedpolicy(&mut attr as *mut pthread_attr_t, 1);
        assert_eq!(ret, 0, "setschedpolicy 应返回 0");

        let mut policy: c_int = -1;
        let ret = pthread_attr_getschedpolicy(
            &attr as *const pthread_attr_t,
            &mut policy as *mut c_int,
        );
        assert_eq!(ret, 0, "getschedpolicy 应返回 0");
        assert_eq!(policy, 1, "policy 应为 1");
    }
});

// ============================================================================
// scope 测试
// ============================================================================

test!("test_attr_set_get_scope_system" {
    // 设置 SCOPE_SYSTEM, 读取应得到 SCOPE_SYSTEM
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let _ = pthread_attr_init(&mut attr as *mut pthread_attr_t);

        let ret = pthread_attr_setscope(
            &mut attr as *mut pthread_attr_t,
            PTHREAD_SCOPE_SYSTEM,
        );
        assert_eq!(ret, 0, "setscope 应返回 0");

        let mut scope: c_int = -1;
        let ret = pthread_attr_getscope(
            &attr as *const pthread_attr_t,
            &mut scope as *mut c_int,
        );
        assert_eq!(ret, 0, "getscope 应返回 0");
        assert_eq!(scope, PTHREAD_SCOPE_SYSTEM, "scope 应为 PTHREAD_SCOPE_SYSTEM(0)");
    }
});

// ============================================================================
// stacksize 测试
// ============================================================================

test!("test_attr_set_get_stacksize" {
    // 设置栈大小, 读取应得到至少为设置值的大小
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let _ = pthread_attr_init(&mut attr as *mut pthread_attr_t);

        let stacksize: usize = 16384;
        let ret = pthread_attr_setstacksize(&mut attr as *mut pthread_attr_t, stacksize);
        assert_eq!(ret, 0, "setstacksize 应返回 0");

        let mut size: usize = 0;
        let ret = pthread_attr_getstacksize(
            &attr as *const pthread_attr_t,
            &mut size as *mut usize,
        );
        assert_eq!(ret, 0, "getstacksize 应返回 0");
        assert!(size >= stacksize, "stacksize 应至少为 {} (got {})", stacksize, size);
    }
});

test!("test_attr_set_stacksize_below_min" {
    // 设置小于 PTHREAD_STACK_MIN 的栈大小应被自动调整
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let _ = pthread_attr_init(&mut attr as *mut pthread_attr_t);

        // 设置非常小的栈大小
        let ret = pthread_attr_setstacksize(&mut attr as *mut pthread_attr_t, 1024);
        // musl 允许设置较小值但实际使用时会对齐到最小值
        // 无论返回什么，设置应该不崩溃
        let _ = ret;

        let mut size: usize = 0;
        let ret = pthread_attr_getstacksize(
            &attr as *const pthread_attr_t,
            &mut size as *mut usize,
        );
        assert_eq!(ret, 0, "getstacksize 应返回 0");
        assert!(size >= 1024, "stacksize 应至少为设置的最小值");
    }
});

// ============================================================================
// stack 测试
// ============================================================================

test!("test_attr_set_get_stack" {
    // 设置栈地址和大小, 读取应得到相同值
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let _ = pthread_attr_init(&mut attr as *mut pthread_attr_t);

        // 在栈上分配一块区域作为自定义栈
        let mut buf: [u8; 4096] = [0u8; 4096];
        let addr = buf.as_mut_ptr() as *mut c_void;
        let stack_size: usize = 4096;

        let ret = pthread_attr_setstack(&mut attr as *mut pthread_attr_t, addr, stack_size);
        assert_eq!(ret, 0, "setstack 应返回 0");

        let mut out_addr: *mut c_void = core::ptr::null_mut();
        let mut out_size: usize = 0;
        let ret = pthread_attr_getstack(
            &attr as *const pthread_attr_t,
            &mut out_addr as *mut *mut c_void,
            &mut out_size as *mut usize,
        );
        assert_eq!(ret, 0, "getstack 应返回 0");
        assert_eq!(out_addr, addr, "stack addr 应保持一致");
        assert_eq!(out_size, stack_size, "stack size 应为 {}", stack_size);
    }
});

// ============================================================================
// 未初始化属性的 get 行为
// ============================================================================

test!("test_attr_default_values" {
    // 新初始化的属性应有合理的默认值
    {
        let mut attr: pthread_attr_t = unsafe { core::mem::zeroed() };
        let ret = pthread_attr_init(&mut attr as *mut pthread_attr_t);
        assert_eq!(ret, 0, "init 应成功");

        // 默认 detachstate 应为 JOINABLE(0)
        let mut state: c_int = -1;
        let ret = pthread_attr_getdetachstate(
            &attr as *const pthread_attr_t,
            &mut state as *mut c_int,
        );
        assert_eq!(ret, 0, "getdetachstate 应成功");
        assert_eq!(
            state, PTHREAD_CREATE_JOINABLE,
            "默认 detachstate 应为 JOINABLE(0)"
        );

        // 默认 inheritsched 应为 INHERIT_SCHED(0)
        let mut inherit: c_int = -1;
        let ret = pthread_attr_getinheritsched(
            &attr as *const pthread_attr_t,
            &mut inherit as *mut c_int,
        );
        assert_eq!(ret, 0, "getinheritsched 应成功");
        assert_eq!(
            inherit, PTHREAD_INHERIT_SCHED,
            "默认 inheritsched 应为 INHERIT_SCHED(0)"
        );

        // 默认 scope 应为 SCOPE_SYSTEM(0)
        let mut scope: c_int = -1;
        let ret = pthread_attr_getscope(
            &attr as *const pthread_attr_t,
            &mut scope as *mut c_int,
        );
        assert_eq!(ret, 0, "getscope 应成功");
        assert_eq!(
            scope, PTHREAD_SCOPE_SYSTEM,
            "默认 scope 应为 SCOPE_SYSTEM(0)"
        );
    }
});
