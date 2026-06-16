# thrd_sleep.c 规约

> musl libc 的 C11 线程睡眠函数实现。基于系统调用 `clock_nanosleep` 实现指定时长的阻塞。

---

## 依赖图

```
thrd_sleep
  └─> __clock_nanosleep(CLOCK_REALTIME, 0, req, rem)  — see internal (内部调用)
```

---

## 函数规约

### 1. thrd_sleep

```c
int thrd_sleep(const struct timespec *req, struct timespec *rem);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.5.6)

#### Intent

使调用线程阻塞至少 `req` 指定的时间。若阻塞被信号中断，剩余时间写入 `rem`。使用 `CLOCK_REALTIME` 时钟，即使系统实时时钟被调整，睡眠时长不受影响（通过 `TIMER_ABSTIME` 标志为 0 即相对时间实现）。

#### 前置条件

- `req != NULL`，指定睡眠时长
- `req->tv_sec >= 0`，`req->tv_nsec` 在 `[0, 999999999]` 范围内
- `rem` 可为 `NULL`（不关心中断后剩余时间）或指向有效 `struct timespec`

#### 后置条件

- Case 1 完整睡眠完成（`ret == 0`）：调用线程已阻塞至少 `req` 时长，返回 0
- Case 2 被信号中断（`ret == -EINTR`）：睡眠提前结束。若 `rem != NULL`，`*rem` 包含剩余未睡眠的时间。返回 -1（C11 标准指定）
- Case 3 其他错误（参数无效等）：返回 -2

#### 系统算法

```
thrd_sleep(req, rem):
  1. // __clock_nanosleep 返回负 errno 值 (内部musl约定)
  2. ret = -__clock_nanosleep(CLOCK_REALTIME, 0, req, rem)
     // flag=0 (相对时间), CLOCK_REALTIME
  3. switch (ret):
       case 0:      return 0    // 完整睡眠
       case -EINTR: return -1   // 信号中断 (C11 指定)
       default:     return -2   // 其他错误
```

#### 不变量

- 返回值 0 / -1 / -2 遵循 C11 标准（注意 *非* thrd_success/thrd_error 枚举值）
- 睡眠时长以 `CLOCK_REALTIME` 为基准

#### 依赖

- `__clock_nanosleep()` — musl 内部时钟睡眠实现，返回负 errno 值（见 `syscall.h` 或内部实现）
- `CLOCK_REALTIME` — POSIX 实时时钟 ID（来自 `<time.h>`）
- `EINTR` — errno 值，系统调用被信号中断
- `struct timespec` — POSIX 时间结构体 `{ time_t tv_sec; long tv_nsec; }`（来自 `<time.h>`）
