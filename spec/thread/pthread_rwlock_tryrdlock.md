# pthread_rwlock_tryrdlock.c 规约

> musl libc 读写锁非阻塞式读加锁函数。尝试获取读锁，若无法立即获取则返回 `EBUSY`。

---

## 依赖图

```
pthread_rwlock_tryrdlock (Public API, weak_alias)
  └─> __pthread_rwlock_tryrdlock (Internal)
        ├─> a_cas (原子 compare-and-swap)  [来自 internal/atomic.h]
```

---

## 函数规约

### 1. __pthread_rwlock_tryrdlock

```c
int __pthread_rwlock_tryrdlock(pthread_rwlock_t *rw);
```

[Visibility]: Internal (不导出) — musl 内部实现，通过 `weak_alias` 作为 `pthread_rwlock_tryrdlock` 对外暴露，同时被 `__pthread_rwlock_timedrdlock` 内部调用

### 2. pthread_rwlock_tryrdlock

```c
int pthread_rwlock_tryrdlock(pthread_rwlock_t *rw);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出（POSIX），是 `__pthread_rwlock_tryrdlock` 的弱别名

#### Intent

以非阻塞方式尝试获取读锁。若锁未被写持有且读者数未达上限，则原子地递增读者计数并成功返回；否则立即返回错误而不阻塞。

#### 前置条件

- `rw != NULL`，指向已初始化的读写锁

#### 后置条件

- Case 1 成功：
  - `_rw_lock` 原子递增 1
  - 调用线程持有读锁
  - 返回 0
- Case 2 写锁被持有（`_rw_lock` 的高位表示写锁或有等待者且锁为写状态）：返回 `EBUSY`
- Case 3 读者计数已达上限 `0x7ffffffe`（即 `INT32_MAX - 1`）：
  - `cnt == 0x7fffffff`：返回 `EBUSY`（实际上是写锁标记位）
  - `cnt == 0x7ffffffe`：返回 `EAGAIN`（读者数溢出保护）

#### 系统算法

```
__pthread_rwlock_tryrdlock(rw):
  1. do:
       val = rw->_rw_lock                  原子读取当前锁值
       cnt = val & 0x7fffffff              提取低 31 位：当前读者数
       if (cnt == 0x7fffffff) return EBUSY 值为 INT32_MAX，写锁或 overflow
       if (cnt == 0x7ffffffe) return EAGAIN 已达最大读者数
     while (a_cas(&rw->_rw_lock, val, val+1)  != val)
      原子 CAS 尝试递增读者计数，失败则重试
  2. return 0
```

关键实现细节：
- `val & 0x7fffffff` 提取低 31 位作为读者计数，忽略 bit 31 的等待者标志
- `0x7fffffff` (INT32_MAX) 被保留为写锁标记，因此最大读者数为 `0x7ffffffe`
- `EAGAIN` 是 musl 特有的扩展：在 glibc 等实现中可能返回 `EAGAIN` 表示递归读锁过多

#### 不变量

- 读者计数始终满足 `0 <= cnt <= 0x7ffffffe`（有效范围内）
- 写锁标记 `0x7fffffff` 与读者计数互斥

#### 依赖

- `a_cas(volatile int *p, int t, int s)` — 原子 compare-and-swap，若 `*p == t` 则设 `*p = s` 并返回旧值（来自 `internal/atomic.h`）
- `EBUSY` / `EAGAIN` — 错误码常量（来自 `<errno.h>` / `<bits/errno.h>`）
- `weak_alias` — 弱符号别名宏（来自 `src/include/features.h`）
