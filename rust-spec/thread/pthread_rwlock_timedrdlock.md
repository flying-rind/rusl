# pthread_rwlock_timedrdlock -- Rust 接口归约

## 原始 C 接口

```c
int __pthread_rwlock_timedrdlock(pthread_rwlock_t *restrict rw, const struct timespec *restrict at);  // Internal — musl 内部主实现
int pthread_rwlock_timedrdlock(pthread_rwlock_t *restrict rw, const struct timespec *restrict at);    // User — __pthread_rwlock_timedrdlock 的 weak_alias
```

---

## Rust 外部 ABI 接口

```rust
// musl 中 __pthread_rwlock_timedrdlock 是主实现，pthread_rwlock_timedrdlock 是其 weak_alias
// rusl 必须同时导出两者
pub extern "C" fn __pthread_rwlock_timedrdlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int;

pub extern "C" fn pthread_rwlock_timedrdlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int {
    __pthread_rwlock_timedrdlock(rw, at)
}
```

---

## 意图

以阻塞方式获取读写锁的读锁，带有绝对超时。若可立即加锁，直接返回；否则在自适应自旋后进入 futex 等待，直到超时或成功获取。

## 前置条件

- `rw` 为非空指针（`!rw.is_null()`），指向已初始化的读写锁
- `at` 为 NULL 表示无限等待，或指向表示绝对超时时刻的 `struct timespec`（基于 `CLOCK_REALTIME`）

## 后置条件

- Case 1 成功: 返回 0，调用线程持有读锁，`_rw_lock` 递增 1
- Case 2 超时: 返回 `ETIMEDOUT`
- Case 3 被信号中断: 返回 `EINTR`
- Case 4 读者数达上限: 返回 `EAGAIN`（由 `__pthread_rwlock_tryrdlock` 返回）
- Case 5 写锁被持有且立即超时: 返回 `EBUSY`

## 不变量

- `_rw_waiters` 精确反映当前 futex 等待中的线程数（自旋期间的瞬时非精确不计）
- 等待者标志 bit 31 在有等待者时被设置
- futex wait 地址始终是 `&rw._rw_lock`

## 算法

```
__pthread_rwlock_timedrdlock(rw, at):
  1. r = __pthread_rwlock_tryrdlock(rw)       尝试立即获取读锁
     if r != EBUSY: return r                    成功 (0 或 EAGAIN) 直接返回

  2. 自适应自旋 (最多 100 次):
     while spins > 0 && rw._rw_lock != 0 && rw._rw_waiters == 0:
         core::hint::spin_loop()                PAUSE 指令，替代 C 的 a_spin()
         spins -= 1
     条件: 锁被持有 且 没有其他等待者
     目的: 在短临界区场景下避免 futex 系统调用开销

  3. 重试 + futex 等待循环:
     while __pthread_rwlock_tryrdlock(rw) == EBUSY:
         val = atomic_load(&rw._rw_lock, Ordering::Acquire)
         // 快速路径：若锁空闲或读者未满，跳回循环顶部重试 tryrdlock
         if val == 0 || (val & 0x7fffffff) != 0x7fffffff:
             continue
         // 慢路径：设置等待者标志并进入 futex 等待
         t = val | 0x80000000                   设置等待者标志位 (bit 31)
         atomic_fetch_add(&rw._rw_waiters, 1)   递增等待者计数
         if compare_exchange(&rw._rw_lock, val, t).is_err():
             // CAS 失败说明锁值已变，递减等待计数后重试
             atomic_fetch_sub(&rw._rw_waiters, 1)
             continue
         // 进入 futex 等待
         r = __timedwait(&rw._rw_lock, t, CLOCK_REALTIME, at, rw._rw_shared ^ 128)
         atomic_fetch_sub(&rw._rw_waiters, 1)   递减等待者计数
         if r != 0 && r != EINTR:
             return r                            非 EINTR 错误（如 ETIMEDOUT）直接返回
         // EINTR 情况：回到循环开头重试 tryrdlock

  4. return 0                                    成功
```

## 关键设计细节

**等待者标志位 (bit 31)**:
`rw._rw_lock` 的低 31 位存储读者计数或 `0x7fffffff`（写锁），bit 31 作为等待者标志。当锁上存在等待者时，唤醒操作需使用 `FUTEX_WAKE` 而非简单修改内存。

**自适应自旋**:
在进入代价高昂的 futex 系统调用之前，短暂自旋（`core::hint::spin_loop()` / `PAUSE` 指令）以捕获短临界区。仅当 `_rw_lock != 0`（锁被持有）且 `_rw_waiters == 0`（无其他等待者）时自旋，避免在已有等待者排队时做无谓自旋。

**`continue` 快速路径**:
在 futex 循环内部，若检测到 `_rw_lock == 0`（锁空闲）或 `(val & 0x7fffffff) != 0x7fffffff`（读者未满），直接 `continue` 回到循环顶部调用 `tryrdlock`，避免不必要的 `__timedwait` 调用。

**futex private 标志**:
`priv = rw._rw_shared ^ 128`:
- 若 `_rw_shared == 128`（进程私有）: `128 ^ 128 = 0`，表示进程私有模式
- 若 `_rw_shared == 0`（进程共享）: `0 ^ 128 = 128`，表示进程共享模式

## Rust 内部辅助函数（模块私有，不对外暴露）

```rust
// 自适应自旋：等待短临界区释放
pub(crate) fn rwlock_spin(rw: &pthread_rwlock_t, spins: &mut i32) {
    while *spins > 0 && rw.lock_val() != 0 && rw.waiters() == 0 {
        core::hint::spin_loop();
        *spins -= 1;
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_rwlock_tryrdlock      // 依赖1: 非阻塞读加锁（见 pthread_rwlock_tryrdlock）
  __timedwait(addr: *mut c_int, val: c_int, clk: clockid_t, at: *const timespec, priv: c_int) -> c_int;
                                  // 依赖2: futex 等待（来自内部 pthread 模块）
  core::sync::atomic::AtomicI32   // 依赖3: 原子类型
  core::sync::atomic::Ordering    // 依赖4: 原子操作内存顺序
  core::hint::spin_loop           // 依赖5: CPU 自旋/暂停指令，替代 C 的 a_spin()

Predefined Constants:
  EBUSY                           // 来自 errno 模块
  EAGAIN                          // 来自 errno 模块
  EINTR                           // 来自 errno 模块
  ETIMEDOUT                       // 来自 errno 模块
  CLOCK_REALTIME                  // 来自 time 模块

Predefined Types:
  pthread_rwlock_t                // 来自 crate 内部类型定义
  timespec                        // 来自 crate 内部类型定义
  c_int                           // 来自 core::ffi

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __pthread_rwlock_timedrdlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int;
                                 // 主实现符号，必须导出（被 __pthread_rwlock_rdlock 调用）
  pub extern "C" fn pthread_rwlock_timedrdlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int;
                                 // 用户可见符号，POSIX 标准接口，ABI 兼容

Internal Interface:
  pub(crate) fn rwlock_spin(rw: &pthread_rwlock_t, spins: &mut i32);
                                 // 自适应自旋辅助函数，供 crate 内部使用
