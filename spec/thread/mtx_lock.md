# mtx_lock.c 规约

> musl libc 的 C11 互斥锁加锁函数实现。对普通互斥锁使用原子 CAS 快速路径，失败或非普通锁则委托 `mtx_timedlock`。

---

## 依赖图

```
mtx_lock
  ├─> a_cas(&m->_m_lock, 0, EBUSY)  — see internal/atomic.h (原子 CAS)
  ├─> PTHREAD_MUTEX_NORMAL           — see <pthread.h> (POSIX 互斥锁类型)
  └─> mtx_timedlock(m, 0)            — see mtx_timedlock.c (同模块内部转发)
```

---

## 函数规约

### 1. mtx_lock

```c
int mtx_lock(mtx_t *m);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.4.3)

#### Intent

锁定互斥锁 `m`。若互斥锁已被其他线程持有，则阻塞直到锁可用。采用两阶段策略：对普通互斥锁尝试原子快速路径，若失败则委托 `mtx_timedlock`（以 NULL 时间戳表示无限等待）。对递归互斥锁直接委托给 `mtx_timedlock` 处理重入逻辑。

#### 前置条件

- `m != NULL`，指向通过 `mtx_init` 初始化的互斥锁对象
- 互斥锁未被销毁
- 对于普通互斥锁 (`mtx_plain`)：调用线程当前未持有该锁（普通互斥锁不可重入）
- 对于递归互斥锁 (`mtx_recursive`)：调用线程可重入（需配对 `mtx_unlock`）

#### 后置条件

- Case 1 普通互斥锁未被占用（`m->_m_lock == 0`）：`a_cas` 将 `_m_lock` 从 0 原子设为 `EBUSY`，返回 `thrd_success` (0)
- Case 2 普通互斥锁已被占用（`m->_m_lock != 0`）：CAS 失败，委托 `mtx_timedlock(m, 0)` 阻塞等待锁释放
- Case 3 递归互斥锁 (`m->_m_type != PTHREAD_MUTEX_NORMAL`)：直接调用 `mtx_timedlock(m, 0)` 处理
- 返回值为 `mtx_timedlock` 的返回值或 `thrd_success`

#### 系统算法

```
mtx_lock(m):
  1. if (m->_m_type == PTHREAD_MUTEX_NORMAL && !a_cas(&m->_m_lock, 0, EBUSY))
       // 快速路径：普通互斥锁 + 锁空闲 + CAS 成功 (旧值 == 0)
       return thrd_success
  2. // 慢速路径：锁被占用或为非普通互斥锁
     return mtx_timedlock(m, 0)
     // musl 扩展: NULL 时间戳 => 无限期阻塞
```

#### 不变量

- 调用线程在返回时独占互斥锁（对于普通互斥锁）
- 对于递归互斥锁，重入计数在内部正确维护

#### 依赖

- `a_cas(p, t, s)` — 原子比较并交换：若 `*p == t` 则 `*p = s`，始终返回旧值（见 `internal/atomic.h`）
- `EBUSY` — errno 值，用作互斥锁"已锁定"标记
- `PTHREAD_MUTEX_NORMAL` (0) — 普通互斥锁类型常量（见 `<pthread.h>`）
- `mtx_timedlock()` — C11 带超时互斥锁加锁函数（见 `mtx_timedlock.c` spec）
- `_m_type` / `_m_lock` — 互斥锁结构字段访问宏（见 `pthread_impl.h`）
- `thrd_success` — C11 枚举值 `0`
