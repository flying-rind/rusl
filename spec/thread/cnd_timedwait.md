# cnd_timedwait.c 规约

> musl libc 的 C11 条件变量定时等待函数实现。等待条件变量，直至被唤醒或超时到期。

---

## 依赖图

```
cnd_timedwait
  └─> __pthread_cond_timedwait((pthread_cond_t *)c, (pthread_mutex_t *)m, ts)
        — see pthread_impl.h (内部实现)
```

---

## 函数规约

### 1. cnd_timedwait

```c
int cnd_timedwait(cnd_t *restrict c, mtx_t *restrict m, const struct timespec *restrict ts);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.3.6)

#### Intent

原子地释放互斥锁 `m` 并在条件变量 `c` 上阻塞调用线程，直到被 `cnd_signal` / `cnd_broadcast` 唤醒或超时到期。被唤醒后重新获取 `m`。是 `__pthread_cond_timedwait` 的包装器，负责将 POSIX 返回值映射为 C11 返回值。

#### 前置条件

- `c != NULL`，指向通过 `cnd_init` 初始化的条件变量
- `m != NULL`，指向通过 `mtx_init` 初始化的互斥锁
- 调用线程必须已锁定 `m`
- `ts` 指定了绝对时间点的超时（基于 `CLOCK_REALTIME`）

#### 后置条件

- 返回前互斥锁 `m` 已被调用线程重新锁定
- Case 1 被 `cnd_signal` / `cnd_broadcast` 唤醒（`ret == 0`）：返回 `thrd_success` (0)
- Case 2 超时到期且未被唤醒（`ret == ETIMEDOUT`）：返回 `thrd_timedout` (4)
- Case 3 其他错误（如 `EINVAL` 无效参数、`EPERM` 互斥锁不归当前线程所有）：返回 `thrd_error` (2)

#### 系统算法

```
cnd_timedwait(c, m, ts):
  1. ret = __pthread_cond_timedwait((pthread_cond_t *)c, (pthread_mutex_t *)m, ts)
  2. switch (ret):
       case 0:         return thrd_success  // 成功被唤醒
       case ETIMEDOUT: return thrd_timedout // 超时
       default:        return thrd_error    // 其他错误 (EINVAL, EPERM)
```

#### 不变量

- 调用线程在阻塞期间不持有互斥锁 `m`，返回时重新持有

#### 依赖

- `__pthread_cond_timedwait()` — POSIX 条件变量定时等待的内部实现（见 `pthread_impl.h`）
- `cnd_t` / `mtx_t` — C11 互斥锁/条件变量类型（见 `<bits/alltypes.h>`）
- `thrd_success` / `thrd_timedout` / `thrd_error` — C11 枚举值 `0` / `4` / `2`
- `ETIMEDOUT` — errno 值，超时错误码
- `restrict` — C 语言关键字，指示指针间无别名
