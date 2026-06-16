//! 线程信号 API 集成测试
//! 测试函数: pthread_kill, pthread_sigmask

use super::*;
use test_framework::test;

// ============================================================================
// pthread_kill 测试
// ============================================================================

test!("test_pthread_kill_self_signal_zero" {
    // 向自己发送信号 0 (空信号, 仅检查线程是否存在)
    {
        let ret = pthread_kill(pthread_self(), 0);
        assert_eq!(ret, 0, "pthread_kill(self, 0) 应返回 0");
    }
});

test!("test_pthread_kill_invalid_signal" {
    // 发送无效信号号应返回 EINVAL
    {
        let ret = pthread_kill(pthread_self(), -1);
        assert_eq!(ret, EINVAL, "pthread_kill(self, -1) 应返回 EINVAL");
    }
});

test!("test_pthread_kill_self_sigcont" {
    // 向自己发送 SIGCONT (18), 对运行中的进程无影响
    {
        let ret = pthread_kill(pthread_self(), 18); // SIGCONT
        assert_eq!(ret, 0, "pthread_kill(self, SIGCONT) 应返回 0");
    }
});

// ============================================================================
// pthread_sigmask 测试
// ============================================================================

test!("test_pthread_sigmask_query" {
    // SIG_SETMASK = 2, 查询当前信号掩码 (传入 NULL set)
    {
        const SIG_SETMASK: c_int = 2;
        let mut old: sigset_t = sigset_t { __bits: [0u64; 16] };
        // 查询当前掩码: how + old (非空), set (NULL)
        let ret = pthread_sigmask(SIG_SETMASK, core::ptr::null(), &mut old as *mut sigset_t);
        assert_eq!(ret, 0, "sigmask 查询应返回 0");
    }
});

test!("test_pthread_sigmask_set_get" {
    // 设置信号掩码并获取旧值
    {
        const SIG_BLOCK: c_int = 0;
        const SIG_UNBLOCK: c_int = 1;

        let mut old: sigset_t = sigset_t { __bits: [0u64; 16] };

        // 先获取当前掩码
        let ret = pthread_sigmask(0, core::ptr::null(), &mut old as *mut sigset_t);
        assert_eq!(ret, 0, "sigmask 应返回 0");

        // 尝试阻塞 SIGUSR1 (信号 10)
        let mut mask: sigset_t = sigset_t { __bits: [0u64; 16] };
        // 设置 bit 10 (SIGUSR1)
        mask.__bits[0] = 1u64 << 10;

        let ret = pthread_sigmask(SIG_BLOCK, &mask as *const sigset_t, &mut old as *mut sigset_t);
        assert_eq!(ret, 0, "sigmask 阻塞应返回 0");

        // 解除阻塞
        let ret = pthread_sigmask(SIG_UNBLOCK, &mask as *const sigset_t, core::ptr::null_mut());
        assert_eq!(ret, 0, "sigmask 解除应返回 0");
    }
});

test!("test_pthread_sigmask_null_both" {
    // how=0 (SIG_BLOCK), set=NULL, old=NULL 应返回 0
    {
        const SIG_BLOCK: c_int = 0;
        let ret = pthread_sigmask(SIG_BLOCK, core::ptr::null(), core::ptr::null_mut());
        assert_eq!(ret, 0, "sigmask(NULL, NULL) 应返回 0");
    }
});
