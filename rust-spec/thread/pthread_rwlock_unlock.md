# pthread_rwlock_unlock -- Rust 接口归约

## 原始 C 接口

```c
int __pthread_rwlock_unlock(pthread_rwlock_t *rw);    // Internal — musl 内部主实现
int pthread_rwlock_unlock(pthread_rwlock_t *rw);       // User — __pthread_rwlock_unlock 的 weak_alias
```

---

## Rust 外部 ABI 接口

```rust
// musl 中 __pthread_rwlock_unlock 是主实现，pthread_rwlock_unlock 是其 weak_alias
// rusl 必须同时导出两者
pub extern "C" fn __pthread_rwlock_unlock(rw: *mut pthread_rwlock_t) -> c_int;

pub extern "C" fn pthread_rwlock_unlock(rw: *mut pthread_rwlock_t) -> c_int {
    __pthread_rwlock_unlock(rw)
}
```

---

## 意图

释放调用线程持有的读锁或写锁。若锁完全变为空闲且存在等待者，通过 futex 唤醒一个或多个等待线程。

## 前置条件

- `rw` 为非空指针（`!rw.is_null()`），指向已初始化的读写锁
- 调用线程必须持有该读写锁（读锁或写锁）
- 解锁未持有的读写锁是未定义行为

## 后置条件

- Case 1 释放写锁（`cnt == 0x7fffffff`）:
  - `_rw_lock` 原子置 0
  - 若存在等待者 (`waiters > 0`)，通过 `__wake` 唤醒所有等待者
  - 返回 0
- Case 2 释放最后一个读锁（`cnt == 1`）:
  - `_rw_lock` 原子置 0
  - 若存在等待者，唤醒其中一个（一般为写者，防止写饥饿）
  - 返回 0
- Case 3 释放非最后一个读锁（`cnt > 1`）:
  - `_rw_lock` 原子递减 1
  - 不唤醒等待者
  - 返回 0
- 始终返回 0（musl 实现无错误返回）

## 不变量

- 解锁后的 `_rw_lock` 值保持一致，反映正确的剩余锁持有者数量
- `_rw_waiters` 仅在 futex 等待循环内部被修改，解锁时不修改此字段

## 算法

```
__pthread_rwlock_unlock(rw):
  1. priv = rw._rw_shared ^ 128           futex private 标志 (0 或 128)

  2. CAS 循环:
     loop:
         val = atomic_load(&rw._rw_lock, Ordering::Acquire)    原子读取当前锁值
         cnt = val & 0x7fffffff                                 提取低 31 位 = 读者数 / 写锁标记
         waiters = atomic_load(&rw._rw_waiters, ...)
         new = if cnt == 0x7fffffff || cnt == 1 { 0 } else { val - 1 }
           若为写锁 或 最后一个读锁 → new = 0 (完全解锁)
           否则                        → new = val - 1 (递减一个读者)
         if compare_exchange(&rw._rw_lock, val, new, Ordering::AcqRel, Ordering::Acquire).is_ok():
             break
         // CAS 失败，说明 lock 值被其他线程修改，重试

  3. if new == 0 && (waiters > 0 || val < 0):
      若锁已完全解锁 且 (存在等待者 或 val 的 bit 31 置位):
        __wake(&rw._rw_lock, cnt, priv)     唤醒等待者
          cnt: 若为写锁(cnt==0x7fffffff)，唤醒所有等待者
               若为最后一个读锁(cnt==1)，唤醒一个等待者
```

## 关键设计细节

**唤醒策略**:
- `val < 0`（即 bit 31 置位）时检查等待者标志，即使 `waiters == 0` 也会触发唤醒，处理竞态条件
- 唤醒数量 `cnt = 1`（最后一个读锁释放时）: 唤醒一个等待者（通常是写者，防止写饥饿）
- 唤醒数量 `cnt = 0x7fffffff`（写锁释放时）: `__wake` 内部将其转换为 `INT_MAX`，唤醒所有等待者

**CAS 循环的必要性**:
在读取 `val` 和 CAS 尝试之间，锁状态可能被其他核心上的线程修改。CAS 循环确保只有在状态未变时才成功更新。

**`_rw_lock` 递减 vs 置零**:
- 释放一个读锁（非最后一个）: `val - 1`，即读者计数减 1，bit 31 的等待者标志保持不变
- 释放最后一个读锁或写锁: 直接置 0，清除所有标志

## Rust 内部辅助函数（模块私有，不对外暴露）

```rust
// 提取读者计数 / 写锁标记（低 31 位）
pub(crate) fn rwlock_cnt(val: c_int) -> c_int {
    val & 0x7fffffff
}

// 检查是否为写锁
pub(crate) fn is_wrlock(val: c_int) -> bool {
    (val & 0x7fffffff) == 0x7fffffff
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __wake(addr: *mut c_void, cnt: c_int, priv: c_int);
                                  // 依赖1: futex 唤醒（static inline，来自 pthread_impl.h）
                                  // 内部调用 __syscall(SYS_futex, addr, FUTEX_WAKE|priv, cnt)
  core::sync::atomic::AtomicI32   // 依赖2: 原子类型
  core::sync::atomic::Ordering    // 依赖3: 原子操作内存顺序

Predefined Types:
  pthread_rwlock_t                // 来自 crate 内部类型定义
  c_int                           // 来自 core::ffi
  c_void                          // 来自 core::ffi

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __pthread_rwlock_unlock(rw: *mut pthread_rwlock_t) -> c_int;
                                 // 主实现符号，必须导出（musl 内部其他模块调用此符号）
  pub extern "C" fn pthread_rwlock_unlock(rw: *mut pthread_rwlock_t) -> c_int;
                                 // 用户可见符号，POSIX 标准接口，ABI 兼容

Internal Interface:
  pub(crate) fn rwlock_cnt(val: c_int) -> c_int;
                                 // 提取锁计数辅助函数
  pub(crate) fn is_wrlock(val: c_int) -> bool;
                                 // 检查是否为写锁的辅助函数
