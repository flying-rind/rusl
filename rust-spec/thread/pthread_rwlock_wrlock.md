# pthread_rwlock_wrlock -- Rust 接口归约

## 原始 C 接口

```c
int __pthread_rwlock_wrlock(pthread_rwlock_t *rw);    // Internal — musl 内部主实现
int pthread_rwlock_wrlock(pthread_rwlock_t *rw);       // User — __pthread_rwlock_wrlock 的 weak_alias
```

---

## Rust 外部 ABI 接口

```rust
// musl 中 __pthread_rwlock_wrlock 是主实现，pthread_rwlock_wrlock 是其 weak_alias
// rusl 必须同时导出两者
pub extern "C" fn __pthread_rwlock_wrlock(rw: *mut pthread_rwlock_t) -> c_int;

pub extern "C" fn pthread_rwlock_wrlock(rw: *mut pthread_rwlock_t) -> c_int {
    __pthread_rwlock_wrlock(rw)
}
```

---

## 意图

以阻塞方式获取读写锁的写锁。若锁空闲，立即获取；否则阻塞直到锁被释放后获取。本质是 `__pthread_rwlock_timedwrlock` 的无超时版本（`at = NULL(0)` 表示无限等待）。

## 前置条件

- `rw` 为非空指针（`!rw.is_null()`），指向已初始化的读写锁
- 写锁不可重入: 调用线程不能已持有该写锁（否则死锁）

## 后置条件

- Case 1 成功: 返回 0，调用线程独占持有写锁（`_rw_lock = 0x7fffffff`）
- Case 2 失败: 返回非零错误码（仅有 `EINTR`，理论上不会发生因为超时参数为 0）

## 不变量

- 写锁被持有时，`_rw_lock == 0x7fffffff`（可能 bit 31 的等待者标志同时置位）
- 写锁被持有时，不能同时存在任何读者

## 算法

```
__pthread_rwlock_wrlock(rw):
  1. return __pthread_rwlock_timedwrlock(rw, core::ptr::null())   超时指针为 NULL(0) 表示无限阻塞
```

Rust 实现直接委托给 `__pthread_rwlock_timedwrlock`，传入 `core::ptr::null()` 作为超时参数。这是 C 实现中 `__pthread_rwlock_timedwrlock(rw, 0)` 的等价写法。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_rwlock_timedwrlock    // 依赖1: 内部带超时写加锁实现（见 pthread_rwlock_timedwrlock）
  core::ptr::null                 // 依赖2: 获取空指针表示无限等待

Predefined Types:
  pthread_rwlock_t                // 来自 crate 内部类型定义
  c_int                           // 来自 core::ffi

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __pthread_rwlock_wrlock(rw: *mut pthread_rwlock_t) -> c_int;
                                 // 主实现符号，必须导出（musl 内部其他模块调用此符号）
  pub extern "C" fn pthread_rwlock_wrlock(rw: *mut pthread_rwlock_t) -> c_int;
                                 // 用户可见符号，POSIX 标准接口，ABI 兼容
