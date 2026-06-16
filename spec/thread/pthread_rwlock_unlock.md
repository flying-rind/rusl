# pthread_rwlock_unlock.c 规约

> musl libc 读写锁解锁函数。释放调用线程持有的读锁或写锁，并在必要时唤醒等待者。

---

## 依赖图

```
pthread_rwlock_unlock (Public API, weak_alias)
  └─> __pthread_rwlock_unlock (Internal)
        ├─> a_cas(&rw->_rw_lock, val, new)    [来自 internal/atomic.h]
        └─> __wake(&rw->_rw_lock, cnt, priv)  [来自 internal/pthread_impl.h 的 static inline]
              └─> __syscall(SYS_futex, ...)   [系统调用，来自内部]
```

---

## 函数规约

### 1. __pthread_rwlock_unlock

```c
int __pthread_rwlock_unlock(pthread_rwlock_t *rw);
```

[Visibility]: Internal (不导出) — musl 内部实现，通过 `weak_alias` 作为 `pthread_rwlock_unlock` 对外暴露

### 2. pthread_rwlock_unlock

```c
int pthread_rwlock_unlock(pthread_rwlock_t *rw);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出（POSIX），是 `__pthread_rwlock_unlock` 的弱别名

#### Intent

释放调用线程持有的读锁或写锁。若锁完全变为空闲且存在等待者，通过 futex 唤醒一个或多个等待线程。

#### 前置条件

- `rw != NULL`，指向已初始化的读写锁
- 调用线程必须持有该读写锁（读锁或写锁）
- 解锁未持有的读写锁是未定义行为

#### 后置条件

- Case 1 释放写锁（`cnt == 0x7fffffff`）：
  - `_rw_lock` 原子置 0
  - 若存在等待者 (`waiters > 0`)，通过 `__wake` 唤醒所有等待者
  - 返回 0
- Case 2 释放最后一个读锁（`cnt == 1`）：
  - `_rw_lock` 原子置 0
  - 若存在等待者，唤醒其中一个（一般为写者）
  - 返回 0
- Case 3 释放非最后一个读锁（`cnt > 1`）：
  - `_rw_lock` 原子递减 1
  - 不唤醒等待者
  - 返回 0
- 始终返回 0（musl 实现无错误返回）

#### 系统算法

```
__pthread_rwlock_unlock(rw):
  1. priv = rw->_rw_shared ^ 128       futex private 标志 (0 或 128)

  2. do:                                 CAS 循环
       val = rw->_rw_lock              原子读取当前锁值
       cnt = val & 0x7fffffff          提取低 31 位 = 读者数 / 写锁标记
       waiters = rw->_rw_waiters       读取等待者计数
       new = (cnt == 0x7fffffff || cnt == 1) ? 0 : val - 1
         若为写锁 或 最后一个读锁 → new = 0 (完全解锁)
         否则                        → new = val - 1 (递减一个读者)
     while (a_cas(&rw->_rw_lock, val, new) != val)
       CAS 失败则重试

  3. if (!new && (waiters || val < 0))
     若锁已完全解锁 且 (存在等待者 或 val 的 bit 31 置位):
       __wake(&rw->_rw_lock, cnt, priv)
         cnt: 若为写锁(cnt==0x7fffffff)，唤醒所有等待者
              若为最后一个读锁(cnt==1)，唤醒一个等待者
```

#### 关键设计细节

**唤醒策略**:
- `val < 0`（即 bit 31 置位）时检查等待者标志，即使 `waiters == 0` 也会触发唤醒，处理竞态条件。
- 唤醒数量 `cnt = 1`（最后一个读锁释放时）：唤醒一个等待者（通常是写者，防止写饥饿）
- 唤醒数量 `cnt = 0x7fffffff`（写锁释放时）：`__wake` 内部将其转换为 `INT_MAX`，唤醒所有等待者

**CAS 循环的必要性**:
在读取 `val` 和 CAS 尝试之间，锁状态可能被其他核心上的线程修改。CAS 循环确保只有在状态未变时才成功更新。

**`_rw_lock` 递减 vs 置零**:
- 释放一个读锁（非最后一个）: `val - 1`，即读者计数减 1，bit 31 的等待者标志保持不变
- 释放最后一个读锁或写锁: 直接置 0，清除所有标志

#### 不变量

- 解锁后的 `_rw_lock` 值保持一致，反映正确的剩余锁持有者数量
- `_rw_waiters` 仅在 futex 等待循环内部被修改，解锁时不修改此字段

#### 依赖

- `a_cas(volatile int *p, int t, int s)` — 原子 compare-and-swap（来自 `internal/atomic.h`）
- `__wake(volatile void *addr, int cnt, int priv)` — futex 唤醒（`static inline`，定义于 `pthread_impl.h`）
  - 内部调用 `__syscall(SYS_futex, addr, FUTEX_WAKE|priv, cnt)`
- `weak_alias` — 弱符号别名宏（来自 `src/include/features.h`）
