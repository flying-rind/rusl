# sleep.c 规约

> musl libc POSIX 秒级睡眠函数。`sleep` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
sleep (Public)
  └── nanosleep(&tv, &tv) — 高精度睡眠 (<time.h>)
        └── struct timespec — POSIX 时间规范结构 (<time.h>)
```

---

## 函数规约

### sleep

```c
unsigned sleep(unsigned seconds);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

使调用进程暂停执行 `seconds` 秒。如果睡眠被信号处理器中断，则返回剩余的未睡眠秒数。

该实现基于 `nanosleep()` 构建（而非 `SYS_alarm` + `SYS_pause` 或 `SYS_nanosleep` 系统调用），使用 `struct timespec` 实现精确的秒级睡眠。若 `nanosleep` 被信号中断，`tv` 参数（传入时同时作为 `&tv` 输出）将被内核更新为剩余时间，`sleep` 直接返回剩余秒数。

#### 前置条件

- `seconds`: 要睡眠的秒数。0 表示不睡眠（但可能让出 CPU）

#### 后置条件

- **Case 1 完整睡眠**
  - 调用进程在至少 `seconds` 秒内未执行
  - 实际睡眠时间可能因系统负载和时钟粒度而略长
  - 返回 0

- **Case 2 被信号处理器中断**
  - 返回剩余未睡眠的秒数
  - 调用者可选择再次调用 `sleep(result)` 以完成剩余睡眠

- **Case 3 `seconds == 0`**
  - 立即返回 0（不睡眠，但可能触发上下文切换）

#### 系统算法

```
sleep(seconds):
  tv.tv_sec  = seconds            // 1. 设置秒级睡眠时长
  tv.tv_nsec = 0                  // 2. 纳秒部分为0
  if nanosleep(&tv, &tv):         // 3. 调用 nanosleep 进行高精度睡眠
    return tv.tv_sec              // 4. 被信号中断：返回剩余秒数
  return 0                        // 5. 完整睡眠：返回0
```

注意：`nanosleep` 的 `rem` 参数和 `req` 参数使用了同一个 `tv` 变量 —— 当被信号中断时，内核将剩余时间写回 `tv`，然后 `sleep` 直接读取 `tv.tv_sec` 返回。这种用法利用了 musl/C 中参数按顺序求值的特性：先计算 `&tv` 作为第一个参数的值，再计算第二个 `&tv`，此时内核尚未修改 `tv` 内容。

#### 依赖

- `nanosleep(const struct timespec *, struct timespec *)` — 来自 `<time.h>`，高精度睡眠（纳秒分辨率）
- `struct timespec` — 来自 `<time.h>`，包含 `time_t tv_sec` 和 `long tv_nsec` 的时间结构
