# pthread_rwlock_trywrlock.c 规约

> musl libc 读写锁非阻塞式写加锁函数。尝试获取写锁，若无法立即获取则返回 `EBUSY`。

---

## 依赖图

```
pthread_rwlock_trywrlock (Public API, weak_alias)
  └─> __pthread_rwlock_trywrlock (Internal)
        ├─> a_cas (原子 compare-and-swap)  [来自 internal/atomic.h]
```

---

## 函数规约

### 1. __pthread_rwlock_trywrlock

```c
int __pthread_rwlock_trywrlock(pthread_rwlock_t *rw);
```

[Visibility]: Internal (不导出) — musl 内部实现，通过 `weak_alias` 作为 `pthread_rwlock_trywrlock` 对外暴露，同时被 `__pthread_rwlock_timedwrlock` 内部调用

### 2. pthread_rwlock_trywrlock

```c
int pthread_rwlock_trywrlock(pthread_rwlock_t *rw);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出（POSIX），是 `__pthread_rwlock_trywrlock` 的弱别名

#### Intent

以非阻塞方式尝试获取写锁。仅当锁完全空闲（`_rw_lock == 0`）时才能获取写锁；若锁已被任何读者或写者持有，立即返回 `EBUSY`。

#### 前置条件

- `rw != NULL`，指向已初始化的读写锁

#### 后置条件

- Case 1 成功：
  - `_rw_lock` 原子地从 0 变为 `0x7fffffff`（写锁标记）
  - 调用线程独占持有写锁
  - 返回 0
- Case 2 锁未被完全获取（`_rw_lock != 0`）：返回 `EBUSY`，锁状态不变

#### 系统算法

```
__pthread_rwlock_trywrlock(rw):
  1. if (a_cas(&rw->_rw_lock, 0, 0x7fffffff))
       return EBUSY    CAS 失败意味着锁当前值不为 0
  2. return 0          CAS 成功意味着锁从 0 变为写锁状态
```

关键实现细节：
- `a_cas(&rw->_rw_lock, 0, 0x7fffffff)` 仅在 `_rw_lock == 0`（完全空闲）时成功
- `0x7fffffff` (INT32_MAX) 是写锁标记值，即使等待者标志位置位也不会超过此值
- 写锁不具备可重入性：同一线程重复调用将死锁（musl 符合 POSIX 写锁不可重入语义）

#### 不变量

- 写锁标记 `0x7fffffff` 时，不可能同时存在读者

#### 依赖

- `a_cas(volatile int *p, int t, int s)` — 原子 compare-and-swap，若 `*p == t` 则设 `*p = s`，返回旧值，非零返回值表示 CAS 失败（旧值 != t）（来自 `internal/atomic.h`）
- `EBUSY` — 错误码常量（来自 `<errno.h>` / `<bits/errno.h>`）
- `weak_alias` — 弱符号别名宏（来自 `src/include/features.h`）
