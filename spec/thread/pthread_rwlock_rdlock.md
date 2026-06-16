# pthread_rwlock_rdlock.c 规约

> musl libc 读写锁阻塞式读加锁函数。阻塞调用线程直到成功获取读锁。

---

## 依赖图

```
pthread_rwlock_rdlock (Public API, weak_alias)
  └─> __pthread_rwlock_rdlock (Internal)
        └─> __pthread_rwlock_timedrdlock(rw, 0)  (see pthread_rwlock_timedrdlock.c spec)
```

---

## 函数规约

### 1. __pthread_rwlock_rdlock

```c
int __pthread_rwlock_rdlock(pthread_rwlock_t *rw);
```

[Visibility]: Internal (不导出) — musl 内部实现，通过 `weak_alias` 作为 `pthread_rwlock_rdlock` 对外暴露

### 2. pthread_rwlock_rdlock

```c
int pthread_rwlock_rdlock(pthread_rwlock_t *rw);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出（POSIX），是 `__pthread_rwlock_rdlock` 的弱别名

#### Intent

以阻塞方式获取读写锁的读锁。若锁未被写持有，且读者数未达上限，则立即获取读锁；否则阻塞直到可以获取。当 `at` 为 NULL (=0) 时，`__pthread_rwlock_timedrdlock` 无限等待。

#### 前置条件

- `rw != NULL`，指向已初始化的读写锁

#### 后置条件

- Case 1 成功：返回 0，调用线程持有读锁，`_rw_lock` 递增 1
- Case 2 失败：返回非零错误码（仅有 `EINTR`，理论上不会发生因为超时参数为 0）

#### 系统算法

```
__pthread_rwlock_rdlock(rw):
  1. return __pthread_rwlock_timedrdlock(rw, 0)   超时参数为 0 表示无限阻塞
```

#### 不变量

- 读锁可重入：同一线程可多次获取读锁，每次获取递增 `_rw_lock`

#### 依赖

- `__pthread_rwlock_timedrdlock()` — 内部超时读加锁实现（见 `pthread_rwlock_timedrdlock.c`）
- `weak_alias` — 弱符号别名宏（来自 `src/include/features.h`）
