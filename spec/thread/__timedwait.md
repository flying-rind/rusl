# __timedwait.c 规约

> musl libc 内部带超时的 futex 等待函数。提供基于 futex 的带超时线程等待原语，支持取消点 (cancellation point) 检查。包括对 32 位/64 位时间系统调用的兼容处理。

---

## 依赖图

```
__timedwait
  ├─> __pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &cs)  (see pthread_setcancelstate — 取消状态)
  ├─> __timedwait_cp(addr, val, clk, at, priv)
  │     ├─> __clock_gettime(clk, &to)       (see clock_gettime — POSIX 时钟)
  │     ├─> IS32BIT(x) 宏                   (本地定义)
  │     ├─> CLAMP(x) 宏                  (本地定义)
  │     ├─> __futex4_cp(addr, op, val, to)    (static, 本地定义)
  │     │     ├─> __syscall_cp(SYS_futex_time64, ...)  (see __syscall_cp.c)
  │     │     └─> __syscall_cp(SYS_futex, ...)          (see __syscall_cp.c)
  │     └─> __eintr_valid_flag             (weak alias, 全局变量)
  └─> __pthread_setcancelstate(cs, 0)     (see pthread_setcancelstate — 恢复取消状态)
```

---

## 宏定义

### IS32BIT

```c
#define IS32BIT(x) !((x)+0x80000000ULL>>32)
```

[Visibility]: Internal (不导出) — 文件作用域宏

#### Intent

检测一个值是否可以用 32 位表示。通过加上 `0x80000000ULL` 然后右移 32 位来判断：如果结果为零，说明原值在 32 位有符号整数范围内。

---

### CLAMP

```c
#define CLAMP(x) (int)(IS32BIT(x) ? (x) : 0x7fffffffU+((0ULL+(x))>>63))
```

[Visibility]: Internal (不导出) — 文件作用域宏

#### Intent

将 64 位时间值钳制到 32 位范围。若值在 32 位范围内则保持原值；否则根据符号位钳制到 `INT_MAX` (正溢出) 或 `INT_MIN+1` (负溢出但钳制为无符号表示)。

---

## 全局变量

### __eintr_valid_flag

```c
static volatile int dummy = 0;
weak_alias(dummy, __eintr_valid_flag);
```

[Visibility]: Internal (不导出) — 弱别名，可能被其他模块覆盖

#### Intent

指示是否可能收到有效的 `EINTR` 错误。初始值为 0（默认为无有效 `EINTR`），由信号处理相关代码设置为非零值，用于修正旧内核错误地将 `SA_RESTART` 信号报告为 `EINTR` 的问题。

---

## 函数规约

### 1. __futex4_cp (static)

```c
static int __futex4_cp(volatile void *addr, int op, int val, const struct timespec *to);
```

[Visibility]: Internal (不导出) — 文件作用域静态函数

#### Intent

带取消点的 4 参数 futex 系统调用包装器。处理 64 位/32 位时间戳兼容性：优先尝试 `SYS_futex_time64`，若不支持则回退到 `SYS_futex` 并将时间戳钳制为 32 位。是 `__timedwait_cp` 的底层系统调用接口。

#### 前置条件

- `addr` 非空，指向有效的 futex 字
- `op` 为有效的 futex 操作码
- `to` 可为 NULL（表示无限等待）

#### 后置条件

- Case `SYS_futex == SYS_futex_time64` 或 `!IS32BIT(s)`：仅调用 `__syscall_cp(SYS_futex_time64, ...)`，若成功或返回非 `-ENOSYS` 则直接返回结果
- Case 需要回退：若 `SYS_futex_time64` 返回 `-ENOSYS` 且 `to` 非空，将 `to` 钳制为 32 位时间戳
- 调用 `__syscall_cp(SYS_futex, ...)` 作为主要或回退路径
- 若 `SYS_futex` 返回 `-ENOSYS`，回退到不带 `FUTEX_PRIVATE` 标志的调用（兼容旧内核）
- 返回值为系统调用结果（负数表示 error number）

#### 系统算法

```
__futex4_cp(addr, op, val, to):
    #ifdef SYS_futex_time64:
      s = to ? to->tv_sec : 0
      ns = to ? to->tv_nsec : 0
      r = -ENOSYS
      if (SYS_futex == SYS_futex_time64 || !IS32BIT(s)):
        r = __syscall_cp(SYS_futex_time64, addr, op, val, to ? (long long[]){s, ns} : 0)
      if (SYS_futex == SYS_futex_time64 || r != -ENOSYS):
        return r
      to = to ? (void *)(long[]){CLAMP(s), ns} : 0
    #endif
    r = __syscall_cp(SYS_futex, addr, op, val, to)
    if (r != -ENOSYS): return r
    return __syscall_cp(SYS_futex, addr, op & ~FUTEX_PRIVATE, val, to)
```

#### 依赖

- `__syscall_cp()` — 带取消点的系统调用包装器（见 `__syscall_cp.c`）
- `SYS_futex` / `SYS_futex_time64` — 系统调用号（见 `syscall.h`）
- `FUTEX_PRIVATE` — futex 私有标志（见 `futex.h`）

---

### 2. __timedwait_cp

```c
int __timedwait_cp(volatile int *addr, int val, clockid_t clk, const struct timespec *at, int priv);
```

[Visibility]: Internal (不导出) — 被 `pthread_impl.h` 声明为 hidden，仅 musl 内部使用

#### Intent

带取消点检查的、支持相对/绝对超时的 futex 等待函数。计算从当前时间到目标时间 `at` 的时间差，若已超时则立即返回 `ETIMEDOUT`。包含旧内核 `EINTR` 错误修正逻辑。

#### 前置条件

- `addr` 非空，指向有效的 futex 字
- `clk` 为有效的时钟 ID (`CLOCK_REALTIME` 或 `CLOCK_MONOTONIC`)
- `at` 可为 NULL（表示无限等待）
- 若 `at` 非空：`at->tv_nsec < 1000000000UL`
- `priv` 为 0 或非 0（非 0 表示私有 futex）

#### 后置条件

- Case `at == NULL`：无限等待，直到被唤醒或取消
- Case `at` 已过期 (`to.tv_sec < 0`)：返回 `ETIMEDOUT`
- Case `at->tv_nsec >= 1000000000UL`：返回 `EINVAL`
- Case `__clock_gettime` 失败：返回 `EINVAL`
- Case `__futex4_cp` 返回 `EINTR` 或 `ETIMEDOUT` 或 `ECANCELED`：返回对应错误
- Case 其他错误：返回 0
- Case `r == EINTR && !__eintr_valid_flag`：修正为返回 0（旧内核兼容）
- 返回 0 表示被正常唤醒，非零值表示错误/超时

#### 系统算法

```
__timedwait_cp(addr, val, clk, at, priv):
  1. if (priv) priv = FUTEX_PRIVATE
  2. if (at):
  3.   if (at->tv_nsec >= 1000000000UL) return EINVAL
  4.   if (__clock_gettime(clk, &to)) return EINVAL
  5.   to.tv_sec = at->tv_sec - to.tv_sec           // 计算剩余时间
  6.   to.tv_nsec = at->tv_nsec - to.tv_nsec
  7.   if (to.tv_nsec < 0): to.tv_sec--; to.tv_nsec += 1000000000
  8.   if (to.tv_sec < 0) return ETIMEDOUT          // 已超时
  9.   top = &to
 10. r = -__futex4_cp(addr, FUTEX_WAIT|priv, val, top)
 11. if (r != EINTR && r != ETIMEDOUT && r != ECANCELED) r = 0
 12. if (r == EINTR && !__eintr_valid_flag) r = 0   // 旧内核 EINTR 修正
 13. return r
```

#### 依赖

- `__clock_gettime()` — 获取当前时间（外部：POSIX clock_gettime）
- `__futex4_cp()` — 4 参数 futex 系统调用（本文件 static 函数）
- `FUTEX_WAIT` / `FUTEX_PRIVATE` — futex 操作码（见 `futex.h`）
- `__eintr_valid_flag` — EINTR 有效性标志（本文件 weak alias）

---

### 3. __timedwait

```c
int __timedwait(volatile int *addr, int val, clockid_t clk, const struct timespec *at, int priv);
```

[Visibility]: Internal (不导出) — 被 `pthread_impl.h` 声明为 hidden，仅 musl 内部使用

#### Intent

`__timedwait_cp` 的无取消点包装器。在调用 `__timedwait_cp` 期间暂时禁用线程取消，确保超时等待过程不会被取消中断。

#### 前置条件

- 与 `__timedwait_cp` 相同
- 调用者处于有效环境中（可调用 `__pthread_setcancelstate`）

#### 后置条件

- 调用期间取消被禁用，返回前恢复原始取消状态
- 返回值与 `__timedwait_cp` 相同
- 即使等待期间有取消请求挂起，也会完成等待并返回结果

#### 系统算法

```
__timedwait(addr, val, clk, at, priv):
  1. __pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &cs)  // 禁用取消
  2. r = __timedwait_cp(addr, val, clk, at, priv)           // 实际等待
  3. __pthread_setcancelstate(cs, 0)                        // 恢复取消状态
  4. return r
```

#### 依赖

- `__timedwait_cp()` — 带取消点的超时等待（本文件函数）
- `__pthread_setcancelstate()` — 设置取消状态（外部：libc pthread 实现）
