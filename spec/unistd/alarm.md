# alarm.c 规约

> musl libc POSIX 闹钟定时器函数。`alarm` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
alarm (Public)
  └── setitimer(ITIMER_REAL, &it, &old) — 设置间隔定时器
        └── struct itimerval — 间隔定时器值结构 (<sys/time.h>)
```

---

## 函数规约

### alarm

```c
unsigned alarm(unsigned seconds);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

设置一个真实时间闹钟，在 `seconds` 秒后向调用进程发送 `SIGALRM` 信号。如果之前已经设置过闹钟且尚未触发，则取消之前的闹钟并返回剩余秒数。调用 `alarm(0)` 取消任何待处理的闹钟，且不发送 `SIGALRM`。

该实现基于 `setitimer(ITIMER_REAL, ...)` 构建，而非直接使用 `SYS_alarm` 系统调用。这样可以获得更高的精度（微秒级）以及更一致的接口语义。

#### 前置条件

- `seconds`: 触发闹钟前需要等待的秒数。0 表示取消之前的闹钟

#### 后置条件

- **Case 1 之前没有待处理的闹钟**
  - 设置新的闹钟，在 `seconds` 秒后触发 `SIGALRM`
  - 返回 0

- **Case 2 之前有待处理的闹钟（被覆盖）**
  - 之前的闹钟被取消
  - 设置新的闹钟，在 `seconds` 秒后触发 `SIGALRM`
  - 返回之前闹钟剩余的秒数（向上取整到最近的秒数）

- **Case 3 `seconds == 0`（取消闹钟）**
  - 取消任何待处理的闹钟
  - 不发送 `SIGALRM`
  - 返回之前闹钟剩余的秒数（向上取整）

#### 系统算法

```
alarm(seconds):
  it.it_value.tv_sec = seconds      // 1. 设置定时器初值 (秒)
  it.it_value.tv_usec = 0           // 2. 微秒部分为0
  setitimer(ITIMER_REAL, &it, &old) // 3. 设置真实时间定时器
  return old.it_value.tv_sec        // 4. 返回旧定时器剩余秒数
       + !!old.it_value.tv_usec     //    若微秒部分非0则+1（向上取整）
```

#### 依赖

- `setitimer(int, const struct itimerval *, struct itimerval *)` — 来自 `<sys/time.h>`，设置间隔定时器
- `ITIMER_REAL` — 来自 `<sys/time.h>`，真实时间定时器类型（以真实流逝时间递减，到期发送 SIGALRM）
- `struct itimerval` — 来自 `<sys/time.h>`，间隔定时器结构体（包含 `it_value` 和 `it_interval` 两个 `struct timeval` 字段）
