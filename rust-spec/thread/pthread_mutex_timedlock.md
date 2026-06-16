# pthread_mutex_timedlock — Rust 接口归约

## 原始 C 接口
```c
int __pthread_mutex_timedlock(pthread_mutex_t *restrict m, const struct timespec *restrict at);
int pthread_mutex_timedlock(pthread_mutex_t *restrict m, const struct timespec *restrict at);  // weak_alias
```

---

## Rust 外部 ABI 接口

```rust
// musl 中 __pthread_mutex_timedlock 是主实现，pthread_mutex_timedlock 是其 weak_alias
// rusl 必须同时导出两者
extern "C" fn __pthread_mutex_timedlock(
    m: *mut pthread_mutex_t,
    at: *const libc_types::timespec
) -> core::ffi::c_int;

extern "C" fn pthread_mutex_timedlock(
    m: *mut pthread_mutex_t,
    at: *const libc_types::timespec
) -> core::ffi::c_int;
```

---

## 意图
以阻塞方式获取互斥锁，但若在指定绝对时间前未能获取则超时返回。整合了快速路径、trylock 首次尝试、自旋等待、PI 专用路径和 timedwait futex 阻塞共五个阶段。是 mutex lock/trylock 的核心引擎。

## 前置条件
- `m` 非空指针（`!m.is_null()`）
- `m` 指向一个已初始化的 `pthread_mutex_t`
- `at` 可以为空指针（无限等待）或指向绝对时间点（基于 `CLOCK_REALTIME`）

## 后置条件
- Case 1 成功获取：
  - 返回值为 `0`
- Case 2 在 `at` 指定时间前未能获取：
  - 返回值为 `ETIMEDOUT`
- Case 3 被信号中断（EINTR）：
  - 在通用路径中：若最终成功则返回 0，否则传播错误
- Case 4 死锁检测（ERRORCHECK 重复加锁）：
  - 返回值为 `EDEADLK`
- Case 5 EOWNERDEAD / ENOTRECOVERABLE：
  - 返回对应的 robust 状态码

## 不变量
- `_m_waiters` 在每次 timedwait 调用前后成对递增/递减
- 自旋阶段仅在 `_m_waiters == 0` 时执行，无竞争场景下避免了 futex 开销
- PI 类型互斥锁的 timedwait 委托给专用的 PI 路径，不与普通类型的循环逻辑混合

## 算法
五阶段策略实现：

```rust
extern "C" fn __pthread_mutex_timedlock(
    m: *mut pthread_mutex_t,
    at: *const libc_types::timespec
) -> core::ffi::c_int {
    unsafe {
        let mref = &*m;
        // 阶段 1: NORMAL 类型快速 CAS 路径
        if (mref.get_type() & 15) == MUTEX_TYPE_NORMAL {
            if atomic_cas(&mref.lock(), 0, EBUSY) == 0 {
                return 0;
            }
        }

        let mtype = mref.get_type();
        let priv_flag = (mtype & 128) ^ 128;

        // 阶段 2: trylock 首次尝试
        let r = __pthread_mutex_trylock(m);
        if r != EBUSY { return r; }

        // 阶段 3: PI 类型委托给专用路径
        if mtype & 8 != 0 {
            return pthread_mutex_timedlock_pi_impl(m, at);
        }

        // 阶段 4: 自旋等待（最多 100 次，仅在没有 waiter 时）
        let mut spins = 100;
        while spins > 0 && mref.get_lock() != 0 && mref.get_waiters() == 0 {
            core::hint::spin_loop();
            spins -= 1;
        }

        // 阶段 5: futex 循环等待
        loop {
            let r = __pthread_mutex_trylock(m);
            if r != EBUSY { return r; }

            let lock_val = mref.get_lock();
            let own = lock_val & 0x3fffffff;

            // spurious wake / robust 快速重试
            if own == 0 && (lock_val == 0 || (mtype & 4) != 0) { continue; }

            // 死锁检测
            if (mtype & 3) == MUTEX_TYPE_ERRORCHECK && own == current_tid() {
                return libc_errcode::EDEADLK;
            }

            // 原子递增 waiters 计数
            mref.inc_waiters();
            let wait_val = lock_val | 0x80000000;
            atomic_cas(&mref.lock(), lock_val, wait_val);

            // futex 阻塞等待
            let r = timedwait(&mref.lock(), wait_val, CLOCK_REALTIME, at, priv_flag);
            mref.dec_waiters();

            if r != 0 && r != libc_errcode::EINTR { return r; }
        }
    }
}
```

内部 Rust 实现复用 `core::sync::atomic` 进行原子操作，`core::hint::spin_loop()` 替代 `a_spin()`，PI 专用路径通过 `linux_futex` 模块的 `FUTEX_LOCK_PI` 操作完成。`__futex4` 内部函数的逻辑（time64 兼容性处理）被整合到 `linux_futex` 模块中，不在本 spec 中重复描述。

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
impl PthreadMutex {
    pub(crate) fn timedlock(&self, timeout: Option<&libc_types::timespec>) -> Result<(), core::ffi::c_int> {
        // 安全包装：带超时加锁
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  atomic_cas / atomic_inc / atomic_dec (内部模块)  // 依赖: 原子操作
  __pthread_mutex_trylock (本模块)       // 依赖: 非阻塞尝试加锁
  current_thread (内部模块)              // 依赖: 获取当前线程 tid（死锁检测）
  timedwait (内部模块)                   // 依赖: futex 带超时阻塞等待
  linux_futex (内部模块)                 // 依赖: PI futex 操作（FUTEX_LOCK_PI, FUTEX_UNLOCK_PI）
  core::hint::spin_loop                  // 依赖: CPU 自旋等待（替代 a_spin）
  core::sync::atomic::Ordering           // 依赖: 原子操作内存排序
  libc_types::timespec                   // 依赖: 时间规格类型

[GUARANTEE]
Exported Interface:
  extern "C" fn __pthread_mutex_timedlock(m: *mut pthread_mutex_t, at: *const libc_types::timespec) -> core::ffi::c_int;
                                 // musl 内部实现符号，rusl 必须导出
  extern "C" fn pthread_mutex_timedlock(m: *mut pthread_mutex_t, at: *const libc_types::timespec) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_mutex_timedlock 符号
Internal Interface:
  impl PthreadMutex::timedlock(&self, timeout: Option<&libc_types::timespec>) -> Result<(), core::ffi::c_int>;
                                 // 安全包装，供 crate 内部使用
