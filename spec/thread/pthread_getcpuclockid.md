# pthread_getcpuclockid.c 规约

> musl libc 获取指定线程的 CPU 时间时钟 ID。通过线程的内核 TID 计算出对应的 `clockid_t`。

---

## 依赖图

```
pthread_getcpuclockid  (无内部依赖)
```

---

## 函数规约

### 1. pthread_getcpuclockid

```c
int pthread_getcpuclockid(pthread_t t, clockid_t *clockid);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

根据给定线程 `t`，将其内核 TID 转换为对应的 CPU 时钟 ID，写入 `*clockid`，供 `clock_gettime` 使用。Linux 内核中每个线程的 CPU 时钟 ID 由 `(-tid - 1) * 8 + 6` 对应的 `CLOCK_THREAD_CPUTIME_ID` 编码得到。

#### 前置条件

- `t != NULL` 且为有效的 `pthread_t`（指向一个已初始化的 `struct pthread`）
- `t->tid` 为有效的内核线程 ID（非零值）
- `clockid != NULL`，指向可写入 `clockid_t` 的内存

#### 后置条件

- `*clockid` 被设置为 `(-(t->tid) - 1) * 8U + 6`，即该线程的 CPU 时钟 ID
- 始终返回 0（成功）
- 无副作用：不修改线程或全局状态

#### 系统算法

```
pthread_getcpuclockid(t, clockid):
  // Linux 内核中 CLOCK_THREAD_CPUTIME_ID 的编码:
  // clockid = (~tid) * 8 + 6 = (-tid - 1) * 8 + 6
  *clockid = (-t->tid - 1) * 8U + 6
  return 0
```

#### 不变量

无。该函数仅读取 `t->tid` 并写入 `*clockid`，不修改任何共享状态。

#### 依赖

- `pthread_impl.h` — `struct pthread` 定义（访问 `t->tid` 字段）
- `clockid_t` — 通过 `<bits/alltypes.h>` 定义
