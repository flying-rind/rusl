//! lock 模块 — rusl 内部自旋锁 (spinlock)。
//!
//! 本模块定义了 `SpinLock` 类型，用于保护内部共享数据结构的临界区。
//! 锁基于 `AtomicI32` 上的原子 CAS 操作实现，支持静态初始化。
//!
//! # 模块可见性
//!
//! `pub(crate)` — 仅在 rusl crate 内部使用。
//!
//! # 有效状态
//!
//! - `0` — 未锁定 (unlocked)
//! - `1` — 已锁定 (locked)

use core::sync::atomic::{AtomicI32, Ordering};

/// rusl 内部自旋锁。
///
/// 对 `AtomicI32` 的零成本包装，提供精确的内存排序控制。
/// 通过 `const fn new()` 支持静态初始化。
///
/// # 使用示例
///
/// ```ignore
/// static MY_LOCK: SpinLock = SpinLock::new();
///
/// // 临界区
/// MY_LOCK.lock();
/// // ... 受保护的代码 ...
/// MY_LOCK.unlock();
/// ```
#[derive(Debug)]
pub struct SpinLock {
    inner: AtomicI32,
}

impl SpinLock {
    /// 创建一个处于未锁定状态的自旋锁实例。
    ///
    /// `const fn` 允许在静态初始化上下文中使用。
    #[inline]
    pub const fn new() -> Self {
        SpinLock {
            inner: AtomicI32::new(0),
        }
    }

    /// 获取自旋锁。
    ///
    /// 使用原子 CAS 自旋等待。调用者不应在持有锁时调用可能阻塞的函数。
    ///
    /// # 前置条件
    ///
    /// - 调用者未持有该锁（禁止同一线程递归加锁）。
    ///
    /// # 后置条件
    ///
    /// - 锁被标记为已持有（`inner` 值为 1）。
    /// - 调用者互斥地访问受该锁保护的共享资源。
    #[inline]
    pub fn lock(&self) {
        // CAS 自旋等待：不断尝试将 0 替换为 1
        // Acquire 语义确保临界区内的操作不会被重排到 lock 之前。
        while self
            .inner
            .compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // 自旋等待，在 x86/arm 上 PAUSE/YIELD 指令由 LLVM 自动生成。
            core::hint::spin_loop();
        }
    }

    #[inline]
    pub fn unlock(&self) {
        // Release 语义确保临界区内的操作在 unlock 之前对所有线程可见。
        self.inner.store(0, Ordering::Release);
    }
}

// 标记为 Send + Sync，因为 SpinLock 提供内部可变性的同步访问。
unsafe impl Send for SpinLock {}
unsafe impl Sync for SpinLock {}

#[cfg(test)]
mod tests {
    use rusl_core::test;
    use super::SpinLock;
    use core::sync::atomic::Ordering;

    test!("spinlock_new_is_unlocked" {
        let lock = SpinLock::new();
        // 新创建的锁应为未锁定状态 (0)
        assert_eq!(lock.inner.load(Ordering::Relaxed), 0);
    });

    test!("spinlock_const_new" {
        const LOCK: SpinLock = SpinLock::new();
        let _ = &LOCK;
    });

    test!("spinlock_lock_unlock_basic" {
        let lock = SpinLock::new();
        // 加锁前状态应为 0
        assert_eq!(lock.inner.load(Ordering::Relaxed), 0);
        lock.lock();
        // 加锁后状态应为 1
        assert_eq!(lock.inner.load(Ordering::Relaxed), 1);
        lock.unlock();
        // 解锁后状态应恢复为 0
        assert_eq!(lock.inner.load(Ordering::Relaxed), 0);
    });

    test!("spinlock_lock_exclusivity" {
        let lock = SpinLock::new();
        lock.lock();
        // 持有锁期间状态为 1，说明已被独占
        assert_eq!(lock.inner.load(Ordering::Relaxed), 1);
        // 再次尝试获取不应成功（模拟：直接读取确认仍为 1）
        // 注意：这里不能真正 lock() 两次（会死锁），只能验证状态。
        lock.unlock();
        assert_eq!(lock.inner.load(Ordering::Relaxed), 0);
    });

    test!("spinlock_lock_twice_sequential" {
        let lock = SpinLock::new();
        lock.lock();
        lock.unlock();
        // 解锁后可以再次加锁
        lock.lock();
        assert_eq!(lock.inner.load(Ordering::Relaxed), 1);
        lock.unlock();
        assert_eq!(lock.inner.load(Ordering::Relaxed), 0);
    });

    test!("spinlock_multiple_cycles" {
        let lock = SpinLock::new();
        for _ in 0..10 {
            lock.lock();
            assert_eq!(lock.inner.load(Ordering::Relaxed), 1);
            lock.unlock();
            assert_eq!(lock.inner.load(Ordering::Relaxed), 0);
        }
    });
}