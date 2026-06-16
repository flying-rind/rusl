# mtx_timedlock.c 规约

> musl libc 的 C11 互斥锁带超时加锁函数实现。是 POSIX `__pthread_mutex_timedlock` 的包装器，负责将 POSIX 返回值映射为 C11 返回值。

---

## 依赖图

```
mtx_timedlock
  └─> __pthread_mutex_timedlock((pthread_mutex_t *)m, ts)  — see pthread_impl.h (内部实现)
```

---

## 函数规约

### 1. mtx_timedlock

```c
int mtx_timedlock(mtx_t *restrict m, const struct timespec *restrict ts);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.4.4)

#### Intent

锁定互斥锁 `m`，若锁被占用则阻塞直至成功或超时。是 `__pthread_mutex_timedlock` 的包装器，将 POSIX 返回值映射为 C11 语义。当 `ts == NULL` 时表示无限期等待（musl 扩展），此扩展被 `mtx_lock` 利用以避免代码重复。

#### 前置条件

- `m != NULL`，指向通过 `mtx_init` 初始化的互斥锁对象
- 互斥锁未被销毁
- 对于普通互斥锁：调用线程当前未持有该锁
- `ts` 指定了绝对时间点的超时（基于 `CLOCK_REALTIME`），或 `NULL` 表示无限期等待

#### 后置条件

- Case 1 锁成功获取（`ret == 0`）：返回 `thrd_success` (0)
- Case 2 超时到期且未获取锁（`ret == ETIMEDOUT`）：返回 `thrd_timedout` (4)，锁未被获取
- Case 3 其他错误（如 `EINVAL` 无效参数、`EAGAIN` 递归锁超过最大重入数等）：返回 `thrd_error` (2)

#### 系统算法

```
mtx_timedlock(m, ts):
  1. ret = __pthread_mutex_timedlock((pthread_mutex_t *)m, ts)
  2. switch (ret):
       default:        return thrd_error    // 其他错误
       case 0:         return thrd_success  // 成功
       case ETIMEDOUT: return thrd_timedout // 超时
```

#### 不变量

- 成功返回时调用线程独占互斥锁，重入计数按互斥锁类型正确更新

#### 依赖

- `__pthread_mutex_timedlock()` — POSIX 互斥锁带超时锁定的内部实现（见 `pthread_impl.h`）
- `mtx_t` — C11 互斥锁类型，等同于 `pthread_mutex_t` 的 typedef（见 `<bits/alltypes.h>`）
- `thrd_success` / `thrd_timedout` / `thrd_error` — C11 枚举值 `0` / `4` / `2`
- `ETIMEDOUT` — errno 值，超时错误码
- `restrict` — C 语言关键字，指示指针间无别名
