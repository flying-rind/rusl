# pthread_rwlock_wrlock.c 规约

> musl libc 读写锁阻塞式写加锁函数。阻塞调用线程直到成功获取写锁。

---

## 依赖图

```
pthread_rwlock_wrlock (Public API, weak_alias)
  └─> __pthread_rwlock_wrlock (Internal)
        └─> __pthread_rwlock_timedwrlock(rw, 0)  (see pthread_rwlock_timedwrlock.c spec)
```

---

## 函数规约

### 1. __pthread_rwlock_wrlock

```c
int __pthread_rwlock_wrlock(pthread_rwlock_t *rw);
```

[Visibility]: Internal (不导出) — musl 内部实现，通过 `weak_alias` 作为 `pthread_rwlock_wrlock` 对外暴露

### 2. pthread_rwlock_wrlock

```c
int pthread_rwlock_wrlock(pthread_rwlock_t *rw);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出（POSIX），是 `__pthread_rwlock_wrlock` 的弱别名

#### Intent

以阻塞方式获取读写锁的写锁。若锁空闲，立即获取；否则阻塞直到锁被释放后获取。本质是 `__pthread_rwlock_timedwrlock` 的无超时版本（`at = 0` 表示无限等待）。

#### 前置条件

- `rw != NULL`，指向已初始化的读写锁
- 写锁不可重入：调用线程不能已持有该写锁（否则死锁）

#### 后置条件

- Case 1 成功：返回 0，调用线程独占持有写锁（`_rw_lock = 0x7fffffff`）
- Case 2 失败：返回非零错误码（仅有 `EINTR`，理论上不会发生因为超时参数为 0）

#### 系统算法

```
__pthread_rwlock_wrlock(rw):
  1. return __pthread_rwlock_timedwrlock(rw, 0)   超时指针为 NULL(0) 表示无限阻塞
```

#### 不变量

- 写锁被持有时，`_rw_lock == 0x7fffffff`（可能 bit 31 的等待者标志同时置位）
- 写锁被持有时，不能同时存在任何读者

#### 依赖

- `__pthread_rwlock_timedwrlock()` — 内部带超时写加锁实现（见 `pthread_rwlock_timedwrlock.c`）
- `weak_alias` — 弱符号别名宏（来自 `src/include/features.h`）
