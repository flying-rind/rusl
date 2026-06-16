# mtx_init.c 规约

> musl libc 的 C11 互斥锁初始化函数实现。根据指定的类型初始化互斥锁为普通锁或递归锁。

---

## 依赖图

```
mtx_init
  ├─> PTHREAD_MUTEX_NORMAL      — see <pthread.h> (POSIX 普通互斥锁类型)
  ├─> PTHREAD_MUTEX_RECURSIVE   — see <pthread.h> (POSIX 递归互斥锁类型)
  └─> mtx_t._m_type / mtx_t._m_lock  — see pthread_impl.h (内部结构字段访问宏)
```

---

## 函数规约

### 1. mtx_init

```c
int mtx_init(mtx_t *m, int type);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.4.1)

#### Intent

根据 `type` 参数将互斥锁初始化为普通互斥锁或递归互斥锁。使用复合字面量初始化所有字段，其中互斥锁类型字段 `_m_type` 按 C11 -> POSIX 映射设置。

#### 前置条件

- `m != NULL`，指向有效的内存位置
- `m` 之前未被初始化（或已被销毁且不再使用）
- `type` 为 `mtx_plain` (0) 或 `mtx_recursive` (1)，通过 `& mtx_recursive` 位测试判断

#### 后置条件

- `*m` 被初始化为：
  - `_m_type = PTHREAD_MUTEX_NORMAL (0)` 当 `type & mtx_recursive == 0` (即 `mtx_plain`)
  - `_m_type = PTHREAD_MUTEX_RECURSIVE (1)` 当 `type & mtx_recursive != 0` (即 `mtx_recursive`)
  - 其他字段（如 `_m_lock`）均为 0
- 始终返回 `thrd_success` (0)
- 注意：`mtx_timed` 标志在初始化时被忽略，超时锁功能由 `mtx_timedlock` 使用 NULL 时间戳扩展直接支持

#### 系统算法

```
mtx_init(m, type):
  1. *m = (mtx_t){
       ._m_type = ((type & mtx_recursive) ? PTHREAD_MUTEX_RECURSIVE : PTHREAD_MUTEX_NORMAL),
     }
  2. return thrd_success
```

#### 不变量

- 零初始化 `_m_lock == 0` 表示互斥锁未被持有
- `_m_type` 决定了 `mtx_lock` / `mtx_trylock` 的锁行为路径

#### 依赖

- `PTHREAD_MUTEX_NORMAL` (0) — POSIX 普通互斥锁类型宏（见 `<pthread.h>`）
- `PTHREAD_MUTEX_RECURSIVE` (1) — POSIX 递归互斥锁类型宏（见 `<pthread.h>`）
- `_m_type` / `_m_lock` — 互斥锁结构字段访问宏（见 `pthread_impl.h`，定义于 `mtx_t.__u.__i[0]` 和 `.__u.__vi[1]`）
- `mtx_recursive` — C11 枚举值 `1`
- `thrd_success` — C11 枚举值 `0`
