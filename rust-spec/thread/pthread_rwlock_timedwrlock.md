# pthread_rwlock_timedwrlock -- Rust 接口归约

## 原始 C 接口

```c
int __pthread_rwlock_timedwrlock(pthread_rwlock_t *restrict rw, const struct timespec *restrict at);  // Internal — musl 内部主实现
int pthread_rwlock_timedwrlock(pthread_rwlock_t *restrict rw, const struct timespec *restrict at);    // User — __pthread_rwlock_timedwrlock 的 weak_alias
```

---

## Rust 外部 ABI 接口

```rust
// musl 中 __pthread_rwlock_timedwrlock 是主实现，pthread_rwlock_timedwrlock 是其 weak_alias
// rusl 必须同时导出两者
pub extern "C" fn __pthread_rwlock_timedwrlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int;

pub extern "C" fn pthread_rwlock_timedwrlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int {
    __pthread_rwlock_timedwrlock(rw, at)
}
```

---

## 意图

以阻塞方式获取读写锁的写锁，带有绝对超时。若可立即加锁（锁完全空闲），直接返回；否则在自适应自旋后进入 futex 等待，直到超时或成功获取。

## 前置条件

- `rw` 为非空指针（`!rw.is_null()`），指向已初始化的读写锁
- `at` 为 NULL 表示无限等待，或指向表示绝对超时时刻的 `struct timespec`（基于 `CLOCK_REALTIME`）

## 后置条件

- Case 1 成功: 返回 0，调用线程独占持有写锁（`_rw_lock = 0x7fffffff`）
- Case 2 超时: 返回 `ETIMEDOUT`
- Case 3 被信号中断: 返回 `EINTR`
- Case 4 锁已被持有且立即超时: 返回 `EBUSY`

## 不变量

- `_rw_waiters` 精确反映当前 futex 等待中的线程数
- 等待者标志 bit 31 在有等待者时被设置
- futex wait 地址始终是 `&rw._rw_lock`
- 写锁被持有时，`_rw_lock == 0x7fffffff`（可能 bit 31 的等待者标志同时置位）

## 算法

```
__pthread_rwlock_timedwrlock(rw, at):
  1. r = __pthread_rwlock_trywrlock(rw)       尝试立即获取写锁
     if r == 0: return 0                         立即成功

  2. 自适应自旋 (最多 100 次):
     while spins > 0 && rw._rw_lock != 0 && rw._rw_waiters == 0:
         core::hint::spin_loop()                PAUSE 指令，替代 C 的 a_spin()
         spins -= 1

  3. 重试 + futex 等待循环:
     while __pthread_rwlock_trywrlock(rw) == EBUSY:
         val = atomic_load(&rw._rw_lock, Ordering::Acquire)
         // 快速路径：若锁空闲，跳回循环顶部重试 trywrlock
         if val == 0:
             continue
         // 慢路径：设置等待者标志并进入 futex 等待
         t = val | 0x80000000                   设置等待者标志位 (bit 31)
         atomic_fetch_add(&rw._rw_waiters, 1)   递增等待者计数
         if compare_exchange(&rw._rw_lock, val, t).is_err():
             atomic_fetch_sub(&rw._rw_waiters, 1)
             continue
         r = __timedwait(&rw._rw_lock, t, CLOCK_REALTIME, at, rw._rw_shared ^ 128)
         atomic_fetch_sub(&rw._rw_waiters, 1)   递减等待者计数
         if r != 0 && r != EINTR:
             return r                            非 EINTR 错误（如 ETIMEDOUT）直接返回
         // EINTR 情况：回到循环开头重试 trywrlock

  4. return 0                                    成功
```

## 与 `__pthread_rwlock_timedrdlock` 的关键差异

| 方面 | 写锁 (timedwrlock) | 读锁 (timedrdlock) |
|------|-------------------|-------------------|
| 快速路径判断 | `_rw_lock == 0` | `_rw_lock == 0` 或 `(val & 0x7fffffff) != 0x7fffffff`（读者未满） |
| try 函数 | `__pthread_rwlock_trywrlock`: CAS(0, 0x7fffffff) | `__pthread_rwlock_tryrdlock`: CAS(val, val+1) |
| 获取条件 | 锁完全空闲 | 无写锁 且 读者未满 |
| 可重入性 | 不可重入（POSIX 规定） | 可重入 |

写锁的 `continue` 条件仅有 `_rw_lock == 0`（锁完全空闲），而读锁版本还可接受已有其他读者的场景。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_rwlock_trywrlock      // 依赖1: 非阻塞写加锁（见 pthread_rwlock_trywrlock）
  __timedwait(addr: *mut c_int, val: c_int, clk: clockid_t, at: *const timespec, priv: c_int) -> c_int;
                                  // 依赖2: futex 等待（来自内部 pthread 模块）
  core::sync::atomic::AtomicI32   // 依赖3: 原子类型
  core::sync::atomic::Ordering    // 依赖4: 原子操作内存顺序
  core::hint::spin_loop           // 依赖5: CPU 自旋指令，替代 C 的 a_spin()

Predefined Constants:
  EBUSY                           // 来自 errno 模块
  EINTR                           // 来自 errno 模块
  ETIMEDOUT                       // 来自 errno 模块
  CLOCK_REALTIME                  // 来自 time 模块

Predefined Types:
  pthread_rwlock_t                // 来自 crate 内部类型定义
  timespec                        // 来自 crate 内部类型定义
  c_int                           // 来自 core::ffi

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __pthread_rwlock_timedwrlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int;
                                 // 主实现符号，必须导出（被 __pthread_rwlock_wrlock 调用）
  pub extern "C" fn pthread_rwlock_timedwrlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int;
                                 // 用户可见符号，POSIX 标准接口，ABI 兼容
