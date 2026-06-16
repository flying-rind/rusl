# pthread_rwlock_timedwrlock.c 规约

> musl libc 读写锁带超时的阻塞式写加锁函数。尝试在指定超时时间内获取写锁，超时后返回 `ETIMEDOUT`。

---

## 依赖图

```
pthread_rwlock_timedwrlock (Public API, weak_alias)
  └─> __pthread_rwlock_timedwrlock (Internal)
        ├─> pthread_rwlock_trywrlock(rw)       (see pthread_rwlock_trywrlock.c spec)
        ├─> a_spin()                           [来自 internal/atomic.h]
        ├─> a_inc(&rw->_rw_waiters)            [来自 internal/atomic.h]
        ├─> a_dec(&rw->_rw_waiters)            [来自 internal/atomic.h]
        ├─> a_cas(&rw->_rw_lock, r, t)         [来自 internal/atomic.h]
        └─> __timedwait(...)                   [来自内部 pthread 模块]
```

---

## 函数规约

### 1. __pthread_rwlock_timedwrlock

```c
int __pthread_rwlock_timedwrlock(pthread_rwlock_t *restrict rw, const struct timespec *restrict at);
```

[Visibility]: Internal (不导出) — musl 内部实现，通过 `weak_alias` 作为 `pthread_rwlock_timedwrlock` 对外暴露，同时被 `__pthread_rwlock_wrlock` 调用

### 2. pthread_rwlock_timedwrlock

```c
int pthread_rwlock_timedwrlock(pthread_rwlock_t *restrict rw, const struct timespec *restrict at);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出（POSIX），是 `__pthread_rwlock_timedwrlock` 的弱别名

#### Intent

以阻塞方式获取读写锁的写锁，带有绝对超时。若可立即加锁（锁完全空闲），直接返回；否则在自适应自旋后进入 futex 等待，直到超时或成功获取。

#### 前置条件

- `rw != NULL`，指向已初始化的读写锁
- `at` 为 NULL 表示无限等待，或指向表示绝对超时时刻的 `struct timespec`（基于 `CLOCK_REALTIME`）

#### 后置条件

- Case 1 成功：返回 0，调用线程独占持有写锁（`_rw_lock = 0x7fffffff`）
- Case 2 超时：返回 `ETIMEDOUT`
- Case 3 被信号中断：返回 `EINTR`
- Case 4 锁已被持有：返回 `EBUSY`（仅当 trywrlock 的直接返回且 `at` 参数使得立即超时时）

#### 系统算法

```
__pthread_rwlock_timedwrlock(rw, at):
  1. r = pthread_rwlock_trywrlock(rw)
     若能立即获取写锁 → return r (0)

  2. 自适应自旋 (最多 100 次):
     while (spins-- && rw->_rw_lock && !rw->_rw_waiters) a_spin()
     条件: 锁被持有 且 没有其他等待者
     目的: 在短临界区场景下避免 futex 系统调用开销

  3. 重试 + futex 等待循环:
     while ((r = __pthread_rwlock_trywrlock(rw)) == EBUSY):
       a) 若 _rw_lock == 0:
          continue  —— 锁刚释放，立刻回到循环顶部重试 trywrlock
       b) t = r | 0x80000000      设置等待者标志位 (bit 31)
       c) a_inc(&rw->_rw_waiters) 递增等待者计数
       d) a_cas(&rw->_rw_lock, r, t)  尝试原子设置等待者标志
       e) r = __timedwait(&rw->_rw_lock, t, CLOCK_REALTIME, at,
                          rw->_rw_shared ^ 128)
          futex 等待: 当 _rw_lock 保持为 t 时睡眠
       f) a_dec(&rw->_rw_waiters) 递减等待者计数
       g) if (r && r != EINTR) return r
          非 EINTR 错误（如 ETIMEDOUT）直接返回
       h) (EINTR 情况) 回到循环开头重试 trywrlock

  4. return r  成功 (0)
```

#### 与 `__pthread_rwlock_timedrdlock` 的关键差异

| 方面 | 写锁 (timedwrlock) | 读锁 (timedrdlock) |
|------|-------------------|-------------------|
| 快速路径快速判断 | `_rw_lock == 0` | `_rw_lock == 0` 或 `(r&0x7fffffff) != 0x7fffffff`（读者未满） |
| try 函数   | `pthread_rwlock_trywrlock`: CAS(0, 0x7fffffff) | `pthread_rwlock_tryrdlock`: CAS(val, val+1) |
| 获取条件 | 锁完全空闲 | 无写锁 且 读者未满 |
| 可重入性 | 不可重入（POSIX 规定） | 可重入 |

写锁的 `continue` 条件仅有 `_rw_lock == 0`（锁完全空闲），而读锁版本还可接受已有其他读者的场景。

#### 不变量

- `_rw_waiters` 精确反映当前 futex 等待中的线程数
- 等待者标志 bit 31 在有等待者时被设置
- futex wait 地址始终是 `&rw->_rw_lock`

#### 依赖

- `pthread_rwlock_trywrlock()` — 非阻塞写加锁（见 `pthread_rwlock_trywrlock.c`）
- `__timedwait(volatile int *, int, clockid_t, const struct timespec *, int)` — futex 等待（来自内部 pthread 模块，声明于 `pthread_impl.h`）
- `a_cas(volatile int *p, int t, int s)` — 原子 compare-and-swap（来自 `internal/atomic.h`）
- `a_inc(volatile int *p)` — 原子递增（来自 `internal/atomic.h`）
- `a_dec(volatile int *p)` — 原子递减（来自 `internal/atomic.h`）
- `a_spin()` — CPU 自旋/暂停指令（来自 `internal/atomic.h`）
- `CLOCK_REALTIME` — 时钟 ID 常量（来自 `<time.h>`）
- `EBUSY` / `EINTR` — 错误码常量（来自 `<errno.h>`）
- `weak_alias` — 弱符号别名宏（来自 `src/include/features.h`）
