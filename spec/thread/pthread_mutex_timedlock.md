# pthread_mutex_timedlock.c 规约

> musl libc pthread 互斥锁带超时加锁。是 mutex lock 的核心引擎，整合了快速路径、trylock、自旋等待、PI 加锁和 timedwait 五个阶段。

---

## 依赖图

```
__pthread_mutex_timedlock
  ├── a_cas(&m->_m_lock, 0, EBUSY)              — NORMAL 类型快速路径
  ├── __pthread_mutex_trylock(m)                 — 首次尝试加锁
  ├── a_spin()                                   — 自旋等待（最多 100 次）
  ├── a_inc / a_dec / a_cas                      — 原子操作
  ├── __timedwait(&m->_m_lock, t, CLOCK_REALTIME, at, priv) — futex 阻塞等待
  ├── __pthread_self()                           — 获取当前线程 tid（死锁检测）
  └── pthread_mutex_timedlock_pi(m, at)          — PI 类型的带超时加锁

pthread_mutex_timedlock_pi
  ├── __futex4(addr, FUTEX_LOCK_PI, ...)        — PI futex 加锁（支持 64 位时间）
  ├── __timedwait(&(int){0}, 0, CLOCK_REALTIME, at, 1) — PI fallback 等待
  ├── __pthread_mutex_trylock(m)                 — PI 成功后获取所有权
  ├── __syscall(SYS_futex, FUTEX_UNLOCK_PI)      — spurious success 恢复
  └── a_store(&m->_m_waiters, -1)               — 标记 spurious waiters

pthread_mutex_timedlock  (weak_alias → __pthread_mutex_timedlock)
```

---

## 内部符号

### __futex4

```c
static int __futex4(volatile void *addr, int op, int val, const struct timespec *to);
```

[Visibility]: Internal (不导出) — 文件内静态函数

封装 futex 系统调用，自动处理 time64 兼容性。当 `SYS_futex_time64` 与 `SYS_futex` 不同时，对 32 位 long 无法表示的秒数自动切换为 time64 系统调用。

#### 前置条件
- `addr`: 有效的 futex 地址
- `op`: futex 操作码（如 `FUTEX_LOCK_PI`）
- `val`: futex 参数值
- `to`: NULL 或指向 timespec 的指针

#### 后置条件
- 返回 futex 系统调用的结果（负数 errno 或 0）

#### 系统算法

```
__futex4(addr, op, val, to):
  1. #ifdef SYS_futex_time64:
       s = to ? to->tv_sec : 0
       ns = to ? to->tv_nsec : 0
       r = -ENOSYS
       if SYS_futex == SYS_futex_time64 || !IS32BIT(s):
         r = __syscall(SYS_futex_time64, addr, op, val, to ? ((long long[]){s, ns}) : 0)
       if SYS_futex == SYS_futex_time64 || r != -ENOSYS: return r
       to = to ? (void *)(long[]){CLAMP(s), ns} : 0   // 回退到旧接口
  2. return __syscall(SYS_futex, addr, op, val, to)
```

#### 宏定义

```c
#define IS32BIT(x) !((x)+0x80000000ULL>>32)      // 检查 x 是否在 32 位范围
#define CLAMP(x) (int)(IS32BIT(x) ? (x) : 0x7fffffffU+((0ULL+(x))>>63))  // 钳制到 int 范围
```

---

### pthread_mutex_timedlock_pi

```c
static int pthread_mutex_timedlock_pi(pthread_mutex_t *restrict m, const struct timespec *restrict at);
```

[Visibility]: Internal (不导出) — 文件内静态函数

使用内核 PI futex 实现带超时的优先级继承互斥锁加锁。

#### 前置条件
- `m->_m_type` 设置了 PI 标志（位 3 = 8）
- `m != NULL`

#### 后置条件

- Case 1 PI futex 成功：
  - 调用 `__pthread_mutex_trylock(m)` 完成所有权设置
  - 返回 trylock 的结果
- Case 2 PI futex 超时：
  - 返回 `ETIMEDOUT`
- Case 3 PI futex 返回 EDEADLK & 类型为 ERRORCHECK：
  - 返回 `EDEADLK`
- Case 4 PI futex 其他错误（如 ENOTRECOVERABLE）：
  - fallback 到 `__timedwait` 自旋直至超时，返回错误

#### 系统算法

```
pthread_mutex_timedlock_pi(m, at):
  1. type = m->_m_type
     priv = (type & 128) ^ 128
     self = __pthread_self()
  2. if !priv: self->robust_list.pending = &m->_m_next
  3. do e = -__futex4(&m->_m_lock, FUTEX_LOCK_PI|priv, 0, at)
     while e == EINTR
  4. if e: self->robust_list.pending = 0
  5. switch e:
     case 0:
       // spurious success 检测
       if !(type&4) && (m->_m_lock & 0x40000000 || m->_m_waiters):
         a_store(&m->_m_waiters, -1)
         __syscall(SYS_futex, &m->_m_lock, FUTEX_UNLOCK_PI|priv)
         self->robust_list.pending = 0
         break  // fall through to timedwait
       m->_m_count = -1  // 标记 PI 获取成功
       return __pthread_mutex_trylock(m)
     case ETIMEDOUT: return e
     case EDEADLK: if (type&3)==PTHREAD_MUTEX_ERRORCHECK: return e
  6. // Fallback: spin-timedwait for recovery
     do e = __timedwait(&(int){0}, 0, CLOCK_REALTIME, at, 1)
     while e != ETIMEDOUT
  7. return e
```

---

### __pthread_mutex_timedlock / pthread_mutex_timedlock

```c
int __pthread_mutex_timedlock(pthread_mutex_t *restrict m, const struct timespec *restrict at);
int pthread_mutex_timedlock(pthread_mutex_t *restrict m, const struct timespec *restrict at);  // weak_alias
```

[Visibility]:
- `__pthread_mutex_timedlock`: Internal (不导出) — musl 内部实现符号
- `pthread_mutex_timedlock`: User — 通过 `<pthread.h>` 对外导出（weak_alias）

#### Intent

以阻塞方式获取互斥锁，但若在指定绝对时间前未能获取则超时返回。整合了快速路径、trylock 首次尝试、自旋等待、PI 专用路径和 timedwait futex 阻塞四阶段策略。

#### 前置条件

- `m != NULL`，指向一个已初始化的 `pthread_mutex_t`
- `at` 可以为 `NULL`（无限等待）或指向绝对时间点（基于 `CLOCK_REALTIME`）

#### 后置条件

- Case 1 成功获取：
  - 返回值为 `0`
- Case 2 在 `at` 指定时间前未能获取：
  - 返回值为 `ETIMEDOUT`
- Case 3 被信号中断（EINTR）：
  - 在通用路径中：若最终成功则返回 0，否则传播错误
- Case 4 死锁检测（ERRORCHECK 重复加锁）：
  - 返回值为 `EDEADLK`
- Case 5 EOWNERDEAD / ENOTRECOVERABLE：
  - 返回对应的 robust 状态码

#### 系统算法

```
__pthread_mutex_timedlock(m, at):
  1. // 阶段 1: NORMAL 快速路径
     if (m->_m_type & 15) == PTHREAD_MUTEX_NORMAL:
       if a_cas(&m->_m_lock, 0, EBUSY) == 0: return 0

  2. type = m->_m_type
     priv = (type & 128) ^ 128

  3. // 阶段 2: trylock 首次尝试
     r = __pthread_mutex_trylock(m)
     if r != EBUSY: return r

  4. // 阶段 3: PI 类型委托给专用路径
     if type & 8: return pthread_mutex_timedlock_pi(m, at)

  5. // 阶段 4: 自旋等待（最多 100 次，仅在没有 waiters 时）
     spins = 100
     while spins-- && m->_m_lock && !m->_m_waiters: a_spin()

  6. // 阶段 5: futex 循环等待
     while (r = __pthread_mutex_trylock(m)) == EBUSY:
       r = m->_m_lock
       own = r & 0x3fffffff
       if !own && (!r || (type&4)): continue   // spurious wake / robust

       // 死锁检测
       if (type&3) == PTHREAD_MUTEX_ERRORCHECK && own == __pthread_self()->tid:
         return EDEADLK

       a_inc(&m->_m_waiters)                    // 递增等待者计数
       t = r | 0x80000000                       // 设置 waiters 标志
       a_cas(&m->_m_lock, r, t)                 // 通知持有者有等待者
       r = __timedwait(&m->_m_lock, t, CLOCK_REALTIME, at, priv)  // futex 阻塞
       a_dec(&m->_m_waiters)                    // 递减等待者计数
       if r && r != EINTR: break                // 超时或致命错误
  7. return r
```

#### 不变量

- `_m_waiters` 在每次 timedwait 调用前后成对递增/递减
- 自旋阶段仅在 `_m_waiters == 0` 时执行，无竞争场景下避免了 futex 开销
- PI 类型互斥锁的 timedwait 委托给 `pthread_mutex_timedlock_pi`，不与普通类型的循环逻辑混合

#### 依赖

| 接口 | 来源 | 说明 |
|------|------|------|
| `a_cas(p, t, s)` | `atomic.h` (内部) | 原子比较交换 |
| `a_spin()` | `atomic.h` (内部) | CPU 自旋等待 |
| `a_inc(p)` | `atomic.h` (内部) | 原子递增 |
| `a_dec(p)` | `atomic.h` (内部) | 原子递减 |
| `a_store(p, v)` | `atomic.h` (内部) | 原子写入 |
| `__pthread_mutex_trylock(m)` | 本模块 (pthread_mutex_trylock.c) | 尝试加锁 |
| `__pthread_self()` | `pthread_impl.h` (内部) | 获取当前线程控制块 |
| `__timedwait(addr, val, clock, at, priv)` | `pthread_impl.h` (内部) | futex 带超时阻塞等待 |
| `__syscall(SYS_futex, ...)` | 内核系统调用 | futex 系统调用 |
| `__syscall(SYS_futex_time64, ...)` | 内核系统调用 | 64 位时间 futex 系统调用 |
| `FUTEX_LOCK_PI` (6) | `futex.h` (内部) | PI futex 加锁 |
| `FUTEX_UNLOCK_PI` (7) | `futex.h` (内部) | PI futex 解锁 |
| `FUTEX_PRIVATE` (128) | `futex.h` (内部) | 进程私有 futex |
| `CLOCK_REALTIME` | `<time.h>` | 实时时钟 |
