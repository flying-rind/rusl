# pthread_rwlock_trywrlock -- Rust 接口归约

## 原始 C 接口

```c
int __pthread_rwlock_trywrlock(pthread_rwlock_t *rw);    // Internal — musl 内部主实现
int pthread_rwlock_trywrlock(pthread_rwlock_t *rw);       // User — __pthread_rwlock_trywrlock 的 weak_alias
```

---

## Rust 外部 ABI 接口

```rust
// musl 中 __pthread_rwlock_trywrlock 是主实现，pthread_rwlock_trywrlock 是其 weak_alias
// rusl 必须同时导出两者
pub extern "C" fn __pthread_rwlock_trywrlock(rw: *mut pthread_rwlock_t) -> c_int;

pub extern "C" fn pthread_rwlock_trywrlock(rw: *mut pthread_rwlock_t) -> c_int {
    __pthread_rwlock_trywrlock(rw)
}
```

---

## 意图

以非阻塞方式尝试获取写锁。仅当锁完全空闲（`_rw_lock == 0`）时才能获取写锁；若锁已被任何读者或写者持有，立即返回 `EBUSY`。

## 前置条件

- `rw` 为非空指针（`!rw.is_null()`），指向已初始化的读写锁

## 后置条件

- Case 1 成功:
  - `_rw_lock` 原子地从 0 变为 `0x7fffffff`（写锁标记）
  - 调用线程独占持有写锁
  - 返回 0
- Case 2 锁未被完全获取（`_rw_lock != 0`）: 返回 `EBUSY`，锁状态不变

## 不变量

- 写锁标记 `0x7fffffff` 时，不可能同时存在读者

## 算法

```
__pthread_rwlock_trywrlock(rw):
  1. 使用 compare_exchange(&rw._rw_lock, 0, 0x7fffffff, Ordering::AcqRel, Ordering::Acquire)
     尝试原子地将锁从 0 改为 0x7fffffff
  2. if CAS 失败:
       return EBUSY              锁当前值不为 0（被读或写持有）
  3. return 0                    CAS 成功，锁从 0 变为写锁状态
```

Rust 实现使用 `core::sync::atomic::AtomicI32::compare_exchange` 替代 C 的 `a_cas`。不使用循环重试，因为该函数是非阻塞语义——如果锁不空闲就立即返回 `EBUSY`。

关键实现细节:
- `compare_exchange(0, 0x7fffffff)` 仅在 `_rw_lock == 0`（完全空闲）时成功
- `0x7fffffff` (INT32_MAX) 是写锁标记值，即使等待者标志位置位也不会超过此值
- 写锁不具备可重入性: 同一线程重复调用将死锁（musl 符合 POSIX 写锁不可重入语义）

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::sync::atomic::AtomicI32   // 依赖1: 原子类型
  core::sync::atomic::Ordering    // 依赖2: 原子操作内存顺序

Predefined Constants:
  EBUSY                           // 来自 errno 模块

Predefined Types:
  pthread_rwlock_t                // 来自 crate 内部类型定义
  c_int                           // 来自 core::ffi

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __pthread_rwlock_trywrlock(rw: *mut pthread_rwlock_t) -> c_int;
                                 // 主实现符号，必须导出（musl 内部其他模块调用此符号）
  pub extern "C" fn pthread_rwlock_trywrlock(rw: *mut pthread_rwlock_t) -> c_int;
                                 // 用户可见符号，POSIX 标准接口，ABI 兼容
