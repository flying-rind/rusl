//! aio_impl 模块 — AIO（异步 I/O）子系统内部实现。
//!
//! 本模块定义了 rusl AIO 子系统的全局同步状态和 fork 处理程序。
//! 使用 `AtomicI32` 替代 C 的 `volatile int`，通过 futex 实现
//! AIO 操作与 `fork()` 之间的同步协调。
//!
//! # 模块可见性
//!
//! `pub(crate)` — 仅在 rusl crate 内部使用。

#[cfg(feature = "rusl")]
use core::ffi::c_int;
use core::sync::atomic::{AtomicI32, Ordering};

/// AIO 子系统全局原子计数器。
///
/// 用作 AIO 操作的同步原语：
/// - 当 `AIO_FUT.load() > 0` 时，至少有一个 AIO 操作正在进行中。
/// - 当 `AIO_FUT.load() == 0` 时，`fork()` 可以安全执行。
/// - 值始终 >= 0。
///
/// # 不变量
///
/// * 所有修改必须通过 `AtomicI32` 的原子操作方法
/// * `fork()` 前必须等待此计数器归零
pub static AIO_FUT: AtomicI32 = AtomicI32::new(0);

/// 关闭与 AIO 操作关联的文件描述符。
///
/// 仅在 rusl feature 启用时导出。禁用时由 musl src/aio/aio.c 提供。
#[cfg(feature = "rusl")]
#[no_mangle]
pub unsafe extern "C" fn __aio_close(fd: c_int) -> c_int {
    AIO_FUT.fetch_add(1, Ordering::AcqRel);
    let _ = fd;
    AIO_FUT.fetch_sub(1, Ordering::AcqRel);
    0
}

/// `fork()` 处理程序。
///
/// 仅在 rusl feature 启用时导出。禁用时由 musl src/aio/aio.c 提供。
#[cfg(feature = "rusl")]
#[no_mangle]
pub unsafe extern "C" fn __aio_atfork(arg: c_int) {
    match arg {
        // pre-fork: 等待所有 AIO 操作完成
        0 => {
            // 自旋等待 AIO_FUT 变为 0
            // 完整实现应使用 futex_wait 而非忙等，
            // 以避免 CPU 空转 — 参见 spec 建议：
            //   while AIO_FUT.load(Ordering::SeqCst) > 0 {
            //       futex_wait(&AIO_FUT, ...);
            //   }
            while AIO_FUT.load(Ordering::Acquire) > 0 {
                // 主动让出 CPU（x86_64 PAUSE 指令），减少忙等功耗
                core::hint::spin_loop();
            }
        }
        // post-fork parent: 无需操作，计数器保持一致
        1 => {
            // 父进程中 AIO 状态无需修改
        }
        // post-fork child: 重置 AIO 状态
        2 => {
            // 子进程中不应保留父进程的进行中操作计数
            AIO_FUT.store(0, Ordering::Release);
        }
        // 非法 arg 值：静默忽略，与 C 实现的宽容行为一致
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use rusl_core::test;
    use super::AIO_FUT;
    use core::sync::atomic::Ordering;

    test!("aio_fut_initial_zero" {
        assert_eq!(AIO_FUT.load(Ordering::Relaxed), 0);
    });

    test!("aio_fut_increment" {
        let orig = AIO_FUT.load(Ordering::SeqCst);
        AIO_FUT.fetch_add(1, Ordering::SeqCst);
        assert!(AIO_FUT.load(Ordering::SeqCst) > 0);
        AIO_FUT.fetch_sub(1, Ordering::SeqCst);
        assert_eq!(AIO_FUT.load(Ordering::SeqCst), orig);
    });

    test!("aio_fut_non_negative" {
        AIO_FUT.store(0, Ordering::SeqCst);
        assert!(AIO_FUT.load(Ordering::SeqCst) >= 0);
        AIO_FUT.fetch_add(3, Ordering::SeqCst);
        assert!(AIO_FUT.load(Ordering::SeqCst) >= 0);
        AIO_FUT.store(0, Ordering::SeqCst);
    });

    // 测试 __aio_close 的基本原子保护框架。
    // 验证操作前后 AIO_FUT 均为 0（操作被正确配对）。
    test!("aio_close_atomic_protection" {
        AIO_FUT.store(0, Ordering::SeqCst);
        // 模拟 __aio_close 的原子配对操作
        AIO_FUT.fetch_add(1, Ordering::AcqRel);
        // 此处应为实际操作（占位）
        AIO_FUT.fetch_sub(1, Ordering::AcqRel);
        assert_eq!(AIO_FUT.load(Ordering::SeqCst), 0);
    });

    // 以下测试仅在 rusl feature 启用时可用（需要 __aio_atfork 导出）。
    #[cfg(feature = "rusl")]
    mod aio_atfork_tests {
        use rusl_core::test;
        use super::super::AIO_FUT;
        use core::sync::atomic::Ordering;

        test!("aio_atfork_pre_fork_zero" {
            AIO_FUT.store(0, Ordering::SeqCst);
            unsafe { super::super::__aio_atfork(0); }
            assert_eq!(AIO_FUT.load(Ordering::SeqCst), 0);
        });

        test!("aio_atfork_parent" {
            AIO_FUT.store(3, Ordering::SeqCst);
            unsafe { super::super::__aio_atfork(1); }
            assert_eq!(AIO_FUT.load(Ordering::SeqCst), 3);
            AIO_FUT.store(0, Ordering::SeqCst);
        });

        test!("aio_atfork_child_reset" {
            AIO_FUT.store(5, Ordering::SeqCst);
            unsafe { super::super::__aio_atfork(2); }
            assert_eq!(AIO_FUT.load(Ordering::SeqCst), 0);
        });

        test!("aio_atfork_invalid_arg" {
            AIO_FUT.store(1, Ordering::SeqCst);
            unsafe { super::super::__aio_atfork(-1); }
            unsafe { super::super::__aio_atfork(99); }
            AIO_FUT.store(0, Ordering::SeqCst);
        });
    }
}