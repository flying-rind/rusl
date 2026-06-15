# ualarm.c 规约

> musl libc BSD/GNU 扩展微秒级间隔定时器函数。`ualarm` 在 `<unistd.h>` 中声明（需要 `_GNU_SOURCE` 或 `_BSD_SOURCE`）。

---

## 依赖图

```
ualarm (Public — BSD/GNU 扩展)
  └── setitimer(ITIMER_REAL, &it, &it_old) — 设置间隔定时器
        └── struct itimerval — 间隔定时器值结构 (<sys/time.h>)
```

---

## 函数规约

### ualarm

```c
unsigned ualarm(unsigned value, unsigned interval);
```

[Visibility]: User — `<unistd.h>` BSD/GNU 扩展函数（需要 `_GNU_SOURCE` 或 `_BSD_SOURCE` 特性宏），用户程序可直接调用

#### Intent

设置一个真实时间闹钟，在 `value` 微秒后第一次触发 `SIGALRM`，之后每隔 `interval` 微秒重复触发。如果之前已经设置过定时器，则取消之前的定时器。调用 `ualarm(0, 0)` 取消任何待处理的定时器。

与 `alarm()`（秒级分辨率，无重复间隔）相比，`ualarm` 提供了微秒级精度和自动重复能力。该实现基于 `setitimer(ITIMER_REAL, ...)` 构建。

#### 前置条件

- `value`: 第一次触发前的等待时间（微秒）。0 表示不设置首次触发
- `interval`: 首次触发后每次重复触发的间隔（微秒）。0 表示只触发一次

#### 后置条件

- **Case 1 之前没有定时器**
  - 设置新的间隔定时器
  - 返回 0

- **Case 2 之前有定时器（被覆盖）**
  - 之前的定时器被取消
  - 设置新的间隔定时器
  - 返回之前定时器距离下次触发所剩余的微秒数

- **Case 3 `value == 0 && interval == 0`（取消定时器）**
  - 取消任何待处理的间隔定时器
  - 返回之前定时器剩余微秒数

#### 系统算法

```
ualarm(value, interval):
  it.it_interval.tv_usec = interval                     // 1. 设置重复间隔（微秒）
  it.it_value.tv_usec    = value                        // 2. 设置首次触发延迟（微秒）
  setitimer(ITIMER_REAL, &it, &it_old)                  // 3. 设置真实时间间隔定时器
  return it_old.it_value.tv_sec * 1000000               // 4. 返回旧定时器剩余微秒数
       + it_old.it_value.tv_usec                        //    = tv_sec * 10^6 + tv_usec
```

注意：`ualarm` 的返回值精确到微秒（而 `alarm` 的返回值向上取整到秒），因为 `ualarm` 本身就是以微秒为单位的接口。

#### 依赖

- `setitimer(int, const struct itimerval *, struct itimerval *)` — 来自 `<sys/time.h>`，设置间隔定时器
- `ITIMER_REAL` — 来自 `<sys/time.h>`，真实时间定时器类型
- `struct itimerval` — 来自 `<sys/time.h>`，间隔定时器结构体（包含 `it_value` 和 `it_interval` 两个 `struct timeval` 字段，每个字段有 `tv_sec` 和 `tv_usec` 成员）

#### 注意事项

- `ualarm` 具有可移植性问题：它是 BSD/SVID 扩展，在 POSIX.1-2001 中标记为已废弃，在 POSIX.1-2008 中被移除。推荐使用 `setitimer` 或 POSIX 定时器代替
- musl 以 `#define _GNU_SOURCE` 编译此文件以启用 GNU 扩展特性
- 与 `alarm()` 共享同一个 `ITIMER_REAL` 定时器，因此调用 `ualarm` 会影响调用 `alarm` 设置的状态，反之亦然
