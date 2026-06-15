# usleep.c 规约

> musl libc BSD/GNU 扩展微秒级睡眠函数。`usleep` 在 `<unistd.h>` 中声明（需要 `_GNU_SOURCE` 或 `_BSD_SOURCE`）。

---

## 依赖图

```
usleep (Public — BSD/GNU 扩展)
  └── nanosleep(&tv, &tv) — 高精度睡眠 (<time.h>)
        └── struct timespec — POSIX 时间规范结构 (<time.h>)
```

---

## 函数规约

### usleep

```c
int usleep(unsigned useconds);
```

[Visibility]: User — `<unistd.h>` BSD/GNU 扩展函数（需要 `_GNU_SOURCE` 或 `_BSD_SOURCE` 特性宏），用户程序可直接调用

#### Intent

使调用进程暂停执行至少 `useconds` 微秒。该实现基于 `nanosleep()` 构建，将微秒值转换为秒+纳秒表示后调用 `nanosleep`。

与 `sleep()`（秒级分辨率，返回剩余秒数）不同，`usleep` 返回 int 错误码（成功返回 0，错误返回 -1），不返回剩余微秒数。如果睡眠被信号中断，调用者无法直接知道剩余时间。

#### 前置条件

- `useconds`: 要睡眠的微秒数。传入 `0` 是合法的（不睡眠，但可能让出 CPU）

#### 后置条件

- **Case 1 完整睡眠**
  - 调用进程在至少 `useconds` 微秒内未执行
  - 实际睡眠时间可能因系统负载和时钟粒度而略长
  - 返回 0

- **Case 2 被信号处理器中断**
  - 返回 -1
  - `errno` 设置为 `EINTR`
  - 无法获知剩余未睡眠的时间（与 `sleep()` 不同，`usleep` 接口不支持返回剩余时间）

- **Case 3 错误**
  - 返回 -1
  - `errno` 设置为相应错误码（如 `EINVAL`，当 `tv_nsec` 超出 [0, 999999999] 范围时；但对合法 `useconds` 不会发生）

#### 系统算法

```
usleep(useconds):
  tv.tv_sec  = useconds / 1000000        // 1. 微秒转秒（整除）
  tv.tv_nsec = (useconds % 1000000)*1000 // 2. 微秒转纳秒（取余 * 1000）
  return nanosleep(&tv, &tv)             // 3. 调用 nanosleep 进行高精度睡眠
```

转换公式：
- `tv_sec = useconds / 1_000_000`
- `tv_nsec = (useconds % 1_000_000) * 1_000`

例如 `usleep(2500000)` → `tv_sec=2, tv_nsec=500000000`（2.5 秒）。

#### 依赖

- `nanosleep(const struct timespec *, struct timespec *)` — 来自 `<time.h>`，高精度睡眠
- `struct timespec` — 来自 `<time.h>`，包含 `time_t tv_sec` 和 `long tv_nsec` 的时间结构

#### 注意事项

- `usleep` 在 POSIX.1-2001 中标记为已废弃，在 POSIX.1-2008 中被移除。推荐使用 `nanosleep(2)` 代替
- musl 以 `#define _GNU_SOURCE` 编译此文件以启用 GNU 扩展特性
- `usleep` 是取消点（cancellation point），因为其内部调用的 `nanosleep` 是取消点
- 与 `sleep()` 的关键区别：`sleep` 返回剩余秒数（需要 `unsigned` 返回类型），`usleep` 返回错误码（`int` 返回类型，0=成功，-1=失败）。这与历史接口语义一致
