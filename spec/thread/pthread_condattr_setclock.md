# pthread_condattr_setclock.c 规约

> musl libc 设置条件变量属性中的时钟类型。

---

## 依赖图

```
pthread_condattr_setclock
  (无内部依赖)
```

---

## 函数规约

### 1. pthread_condattr_setclock

```c
int pthread_condattr_setclock(pthread_condattr_t *a, clockid_t clk);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

设置条件变量超时等待使用的时钟类型（`CLOCK_REALTIME` 或 `CLOCK_MONOTONIC`）到属性对象中。

#### 前置条件

- `a != NULL`，指向有效的 `pthread_condattr_t` 对象
- `clk` 为有效的时钟 ID

#### 后置条件

- Case 1 时钟验证成功：
  - `a->__attr` 的最高位（进程共享标志）保持不变
  - `a->__attr` 的低 31 位设置为 `clk`
  - 返回 `0`
- Case 2 时钟无效（`clk < 0` 或 `clk-2U < 2`，即非 `CLOCK_REALTIME`(=0) 且非 `CLOCK_MONOTONIC`(=1) 且非 `CLOCK_PROCESS_CPUTIME_ID`(=2) 等 musl 支持的时钟）：
  - 返回 `EINVAL`
  - `a->__attr` 不变

#### 系统算法

```
pthread_condattr_setclock(a, clk):
  1. if clk < 0 或 clk-2U < 2 (即 clk 为 2 或 3 等不支持值):
        return EINVAL
  2. a->__attr &= 0x80000000   // 保留最高位（进程共享标志）
  3. a->__attr |= clk           // 设置时钟 ID 到低 31 位
  4. return 0
```

注：`clk-2U < 2` 是一种惯用技巧，当 `clk` 为 `[0, INT_MAX]` 范围内的无符号转换后值 < 2 时通过（即允许 0 和 1）；其余被拒绝。

#### 不变量

- `a->__attr` 最高位的进程共享标志在设置时钟时不可丢失

#### 依赖

- `pthread_condattr_t` — 定义于 `<pthread.h>`，实质为 `struct { unsigned __attr; }`
- `EINVAL` — POSIX 错误码，来自 `<errno.h>`
- `clockid_t` — 来自 `<time.h>`
