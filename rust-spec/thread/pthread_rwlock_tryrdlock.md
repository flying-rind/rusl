# pthread_rwlock_tryrdlock -- Rust 接口归约

## 原始 C 接口

```c
int __pthread_rwlock_tryrdlock(pthread_rwlock_t *rw);    // Internal — musl 内部主实现
int pthread_rwlock_tryrdlock(pthread_rwlock_t *rw);       // User — __pthread_rwlock_tryrdlock 的 weak_alias
```

---

## Rust 外部 ABI 接口

```rust
// musl 中 __pthread_rwlock_tryrdlock 是主实现，pthread_rwlock_tryrdlock 是其 weak_alias
// rusl 必须同时导出两者
pub extern "C" fn __pthread_rwlock_tryrdlock(rw: *mut pthread_rwlock_t) -> c_int;

pub extern "C" fn pthread_rwlock_tryrdlock(rw: *mut pthread_rwlock_t) -> c_int {
    __pthread_rwlock_tryrdlock(rw)
}
```

---

## 意图

以非阻塞方式尝试获取读锁。若锁未被写持有且读者数未达上限，则原子地递增读者计数并成功返回；否则立即返回错误而不阻塞。

## 前置条件

- `rw` 为非空指针（`!rw.is_null()`），指向已初始化的读写锁

## 后置条件

- Case 1 成功:
  - `_rw_lock` 原子递增 1
  - 调用线程持有读锁
  - 返回 0
- Case 2 写锁被持有（`_rw_lock` 的值为 `0x7fffffff`，表示写锁）: 返回 `EBUSY`
- Case 3 读者计数已达上限:
  - `cnt == 0x7fffffff`: 返回 `EBUSY`（实际上是写锁标记位）
  - `cnt == 0x7ffffffe`: 返回 `EAGAIN`（读者数溢出保护，musl 特有扩展）

## 不变量

- 读者计数始终满足 `0 <= cnt <= 0x7ffffffe`（有效范围内）
- 写锁标记 `0x7fffffff` 与读者计数互斥

## 算法

```
__pthread_rwlock_tryrdlock(rw):
  1. loop:
       val = atomic_load(&rw._rw_lock, Ordering::Acquire)    原子读取当前锁值
       cnt = val & 0x7fffffff                                 提取低 31 位：当前读者数
       if cnt == 0x7fffffff { return EBUSY }                 值为 INT32_MAX，写锁或 overflow
       if cnt == 0x7ffffffe { return EAGAIN }                已达最大读者数
       if compare_exchange(&rw._rw_lock, val, val+1, Ordering::AcqRel, Ordering::Acquire).is_ok() {
           return 0                                             CAS 成功，递增读者计数
       }
       // CAS 失败，说明 lock 值被其他线程修改，重试
```

Rust 实现使用 `core::sync::atomic::AtomicI32::compare_exchange` 替代 C 的 `a_cas`。CAS 循环确保在无锁竞争下的正确性。

关键实现细节:
- `val & 0x7fffffff` 提取低 31 位作为读者计数，忽略 bit 31 的等待者标志
- `0x7fffffff` (INT32_MAX) 被保留为写锁标记，因此最大读者数为 `0x7ffffffe`
- `EAGAIN` 是 musl 特有的扩展：在 glibc 等实现中可能返回 `EAGAIN` 表示递归读锁过多

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::sync::atomic::AtomicI32   // 依赖1: 原子类型，替代 C volatile int
  core::sync::atomic::Ordering    // 依赖2: 原子操作内存顺序

Predefined Constants:
  EBUSY                           // 来自 errno 模块
  EAGAIN                          // 来自 errno 模块

Predefined Types:
  pthread_rwlock_t                // 来自 crate 内部类型定义
  c_int                           // 来自 core::ffi

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __pthread_rwlock_tryrdlock(rw: *mut pthread_rwlock_t) -> c_int;
                                 // 主实现符号，必须导出（musl 内部其他模块调用此符号）
  pub extern "C" fn pthread_rwlock_tryrdlock(rw: *mut pthread_rwlock_t) -> c_int;
                                 // 用户可见符号，POSIX 标准接口，ABI 兼容
