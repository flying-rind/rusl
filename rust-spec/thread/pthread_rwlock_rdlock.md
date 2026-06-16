# pthread_rwlock_rdlock -- Rust 接口归约

## 原始 C 接口

```c
int __pthread_rwlock_rdlock(pthread_rwlock_t *rw);    // Internal — musl 内部主实现
int pthread_rwlock_rdlock(pthread_rwlock_t *rw);       // User — __pthread_rwlock_rdlock 的 weak_alias
```

---

## Rust 外部 ABI 接口

```rust
// musl 中 __pthread_rwlock_rdlock 是主实现，pthread_rwlock_rdlock 是其 weak_alias
// rusl 必须同时导出两者
pub extern "C" fn __pthread_rwlock_rdlock(rw: *mut pthread_rwlock_t) -> c_int;

pub extern "C" fn pthread_rwlock_rdlock(rw: *mut pthread_rwlock_t) -> c_int {
    __pthread_rwlock_rdlock(rw)
}
```

---

## 意图

以阻塞方式获取读写锁的读锁。若锁未被写持有，且读者数未达上限，则立即获取读锁；否则阻塞直到可以获取。当 `at` 为 NULL (=0) 时，`__pthread_rwlock_timedrdlock` 无限等待。

## 前置条件

- `rw` 为非空指针（`!rw.is_null()`），指向已初始化的读写锁

## 后置条件

- Case 1 成功: 返回 0，调用线程持有读锁，`_rw_lock` 递增 1
- Case 2 失败: 返回非零错误码（仅有 `EINTR`，理论上不会发生因为超时参数为 0）

## 不变量

- 读锁可重入: 同一线程可多次获取读锁，每次获取递增 `_rw_lock`
- 读锁不会被写锁阻塞的例外: 若当前线程已持有写锁，尝试获取读锁将导致死锁（POSIX 允许但实现不支持）

## 算法

```
__pthread_rwlock_rdlock(rw):
  1. return __pthread_rwlock_timedrdlock(rw, core::ptr::null())   超时参数为 NULL(0) 表示无限阻塞
```

Rust 实现直接委托给 `__pthread_rwlock_timedrdlock`，传入 `core::ptr::null()` 作为超时参数。这是 C 实现中 `__pthread_rwlock_timedrdlock(rw, 0)` 的等价写法。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_rwlock_timedrdlock    // 依赖1: 内部带超时读加锁实现（见 pthread_rwlock_timedrdlock）
  core::ptr::null                 // 依赖2: 获取空指针表示无限等待

Predefined Types:
  pthread_rwlock_t                // 来自 crate 内部类型定义
  c_int                           // 来自 core::ffi

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __pthread_rwlock_rdlock(rw: *mut pthread_rwlock_t) -> c_int;
                                 // 主实现符号，必须导出（musl 内部其他模块调用此符号）
  pub extern "C" fn pthread_rwlock_rdlock(rw: *mut pthread_rwlock_t) -> c_int;
                                 // 用户可见符号，POSIX 标准接口，ABI 兼容
