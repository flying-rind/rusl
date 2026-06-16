# pthread_rwlock_timedrdlock.c 规约

> musl libc 读写锁带超时的阻塞式读加锁函数。尝试在指定超时时间内获取读锁，超时后返回 `ETIMEDOUT`。

---

## 依赖图

```
pthread_rwlock_timedrdlock (Public API, weak_alias)
  └─> __pthread_rwlock_timedrdlock (Internal)
        ├─> pthread_rwlock_tryrdlock(rw)       (see pthread_rwlock_tryrdlock.c spec)
        ├─> a_spin()                           [来自 internal/atomic.h]
        ├─> a_inc(&rw->_rw_waiters)            [来自 internal/atomic.h]
        ├─> a_dec(&rw->_rw_waiters)            [来自 internal/atomic.h]
        ├─> a_cas(&rw->_rw_lock, r, t)         [来自 internal/atomic.h]
        └─> __timedwait(...)                   [来自内部 pthread 模块]
```

---

## 函数规约

### 1. __pthread_rwlock_timedrdlock

```c
int __pthread_rwlock_timedrdlock(pthread_rwlock_t *restrict rw, const struct timespec *restrict at);
```

[Visibility]: Internal (不导出) — musl 内部实现，通过 `weak_alias` 作为 `pthread_rwlock_timedrdlock` 对外暴露，同时被 `__pthread_rwlock_rdlock` 调用

### 2. pthread_rwlock_timedrdlock

```c
int pthread_rwlock_timedrdlock(pthread_rwlock_t *restrict rw, const struct timespec *restrict at);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出（POSIX），是 `__pthread_rwlock_timedrdlock` 的弱别名

#### Intent

以阻塞方式获取读写锁的读锁，带有绝对超时。若可立即加锁，直接返回；否则在自适应自旋后进入 futex 等待，直到超时或成功获取。

#### 前置条件

- `rw != NULL`，指向已初始化的读写锁
- `at` 为 NULL 表示无限等待，或指向表示绝对超时时刻的 `struct timespec`（基于 `CLOCK_REALTIME`）

#### 后置条件

- Case 1 成功：返回 0，调用线程持有读锁，`_rw_lock` 递增 1
- Case 2 超时：返回 `ETIMEDOUT`
- Case 3 被信号中断：返回 `EINTR`
- Case 4 读者数达上限：返回 `EAGAIN`（由 `__pthread_rwlock_tryrdlock` 返回）
- Case 5 写锁被持有：返回 `EBUSY`（仅当 tryrdlock 的直接返回值就是 `EBUSY` 且 `at` 参数使得立即超时时）

#### 系统算法

```
__pthread_rwlock_timedrdlock(rw, at):
  1. r = pthread_rwlock_tryrdlock(rw)
     若能立即获取读锁 → return r (0 或 EAGAIN)

  2. 自适应自旋 (最多 100 次):
     while (spins-- && rw->_rw_lock && !rw->_rw_waiters) a_spin()
     条件: 锁被持有 且 没有其他等待者
     目的: 在短临界区场景下避免 futex 系统调用开销

  3. 重试 + futex 等待循环:
     while ((r = __pthread_rwlock_tryrdlock(rw)) == EBUSY):
       a) 若 _rw_lock 为 0 或读者未满 (低31位 != 0x7fffffff):
          continue  —— 短暂窗口，继续重试
       b) t = r | 0x80000000      设置等待者标志位
       c) a_inc(&rw->_rw_waiters) 递增等待者计数
       d) a_cas(&rw->_rw_lock, r, t)  尝试设置等待者标志
       e) r = __timedwait(&rw->_rw_lock, t, CLOCK_REALTIME, at,
                          rw->_rw_shared ^ 128)
          futex 等待: 当 _rw_lock 保持为 t 时睡眠
       f) a_dec(&rw->_rw_waiters) 递减等待者计数
       g) if (r && r != EINTR) return r
          非 EINTR 错误（如 ETIMEDOUT）直接返回
       h) (EINTR 情况) 回到循环开头重试 tryrdlock

  4. return r  成功 (0)
```

#### 关键设计细节

**等待者标志位 (bit 31)**:
`rw->_rw_lock` 的低 31 位存储读者计数或 `0x7fffffff`（写锁），bit 31 作为等待者标志。当锁上存在等待者时，唤醒操作需使用 `FUTEX_WAKE` 而非简单修改内存。

**自适应自旋**:
在进入代价高昂的 futex 系统调用之前，短暂自旋（`a_spin()` = `PAUSE` 指令）以捕获短临界区。仅当 `_rw_lock != 0`（锁被持有）且 `_rw_waiters == 0`（无其他等待者）时自旋，避免在已有等待者排队时做无谓自旋。

**`continue` 快速路径**:
在 futex 循环内部，若检测到 `_rw_lock == 0`（锁空闲）或 `(r & 0x7fffffff) != 0x7fffffff`（读者未满），直接 `continue` 回到循环顶部调用 `tryrdlock`，避免不必要的 `__timedwait` 调用。

**futex private 标志**:
`priv = rw->_rw_shared ^ 128`:
- 若 `_rw_shared == 128`（进程私有）：`128 ^ 128 = 0`，不设置 `FUTEX_PRIVATE`，由 `__timedwait` 内部处理
- 若 `_rw_shared == 0`（进程共享）：`0 ^ 128 = 128 = FUTEX_PRIVATE` 取反，不设置 private

#### 不变量

- `_rw_waiters` 精确反映当前 futex 等待中的线程数（自旋期间的瞬时非精确不计）
- 等待者标志 bit 31 在有等待者时被设置
- futex wait 地址始终是 `&rw->_rw_lock`

#### 依赖

- `pthread_rwlock_tryrdlock()` — 非阻塞读加锁（见 `pthread_rwlock_tryrdlock.c`）
- `__timedwait(volatile int *, int, clockid_t, const struct timespec *, int)` — futex 等待（来自内部 pthread 模块，声明于 `pthread_impl.h`）
- `a_cas(volatile int *p, int t, int s)` — 原子 compare-and-swap（来自 `internal/atomic.h`）
- `a_inc(volatile int *p)` — 原子递增（来自 `internal/atomic.h`）
- `a_dec(volatile int *p)` — 原子递减（来自 `internal/atomic.h`）
- `a_spin()` — CPU 自旋/暂停指令（来自 `internal/atomic.h`，展开为 `a_barrier()` / `__asm__ volatile("pause":::"memory")`）
- `CLOCK_REALTIME` — 时钟 ID 常量（来自 `<time.h>` / `<bits/alltypes.h>`）
- `EBUSY` / `EAGAIN` / `EINTR` — 错误码常量（来自 `<errno.h>`）
- `weak_alias` — 弱符号别名宏（来自 `src/include/features.h`）
