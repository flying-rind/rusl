# mtx_trylock.c 规约

> musl libc 的 C11 互斥锁尝试加锁函数实现。对普通互斥锁使用原子 CAS 一次尝试，非普通互斥锁委托 `__pthread_mutex_trylock`。

---

## 依赖图

```
mtx_trylock
  ├─> a_cas(&m->_m_lock, 0, EBUSY)        — see internal/atomic.h (原子 CAS)
  ├─> PTHREAD_MUTEX_NORMAL                 — see <pthread.h> (POSIX 互斥锁类型)
  └─> __pthread_mutex_trylock((pthread_mutex_t *)m)  — see pthread_impl.h (内部实现)
```

---

## 函数规约

### 1. mtx_trylock

```c
int mtx_trylock(mtx_t *m);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.4.5)

#### Intent

尝试锁定互斥锁 `m`，但不阻塞。若锁可用则获取并返回成功，若已被占用则立即返回 `thrd_busy`。对普通互斥锁使用快速原子 CAS 路径，对递归互斥锁委托给 `__pthread_mutex_trylock` 内部实现。

#### 前置条件

- `m != NULL`，指向通过 `mtx_init` 初始化的互斥锁对象
- 互斥锁未被销毁

#### 后置条件

- Case 1 普通互斥锁且锁空闲（`m->_m_lock == 0` 且 `a_cas` 成功）：`_m_lock` 原子设为 `EBUSY`，返回 `thrd_success` (0)，锁被获取
- Case 2 普通互斥锁且已被占用（`a_cas` 返回 `EBUSY`）：`(EBUSY & EBUSY) != 0`，返回 `thrd_busy` (1)，锁未获取
- Case 3 非普通互斥锁（递归锁等）：委托 `__pthread_mutex_trylock`，根据返回值映射：
  - `0` -> `thrd_success` (0)
  - `EBUSY` -> `thrd_busy` (1)
  - 其他 -> `thrd_error` (2)

#### 系统算法

```
mtx_trylock(m):
  1. if (m->_m_type == PTHREAD_MUTEX_NORMAL):
       // 快速路径: a_cas 返回 old 值
       // old == 0   => CAS 成功 (锁空闲 -> 设为 EBUSY), 返回 thrd_success
       // old != 0   => CAS 失败 (锁已被占用),  返回 thrd_busy
       return (a_cas(&m->_m_lock, 0, EBUSY) & EBUSY) ? thrd_busy : thrd_success

  2. // 慢速路径: 递归/检错互斥锁
     ret = __pthread_mutex_trylock((pthread_mutex_t *)m)
     switch (ret):
       default:    return thrd_error
       case 0:     return thrd_success
       case EBUSY: return thrd_busy
```

#### 不变量

- 获取锁的线程独占互斥锁直至调用 `mtx_unlock`

#### 依赖

- `a_cas(p, t, s)` — 原子比较并交换：若 `*p == t` 则 `*p = s`，返回旧值（见 `internal/atomic.h`）
- `EBUSY` — errno 值，用作互斥锁"已锁定"标记
- `PTHREAD_MUTEX_NORMAL` (0) — 普通互斥锁类型常量（见 `<pthread.h>`）
- `__pthread_mutex_trylock()` — POSIX 互斥锁非阻塞尝试加锁的内部实现（见 `pthread_impl.h`）
- `_m_type` / `_m_lock` — 互斥锁结构字段访问宏（见 `pthread_impl.h`）
- `thrd_success` / `thrd_busy` / `thrd_error` — C11 枚举值 `0` / `1` / `2`
