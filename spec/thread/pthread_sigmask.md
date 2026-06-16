# pthread_sigmask.c 规约

> musl libc 的线程信号掩码操作函数实现。对 `pthread_sigmask` 的调用包装系统调用 `SYS_rt_sigprocmask`，并清除内部信号位以避免用户观察到内部信号（`SIGCANCEL` / `SIGSYNCCALL`）。

---

## 依赖图

```
pthread_sigmask
  └─> __syscall(SYS_rt_sigprocmask, ...)  — see internal/syscall.h
```

---

## 对外导出函数规约

### 1. pthread_sigmask

```c
int pthread_sigmask(int how, const sigset_t *restrict set, sigset_t *restrict old);
```

[Visibility]: User — 通过 `<signal.h>` / `<pthread.h>` 对外导出 (POSIX)

#### Intent

检查和/或修改调用线程的信号掩码。与 `sigprocmask()` 的功能相同，但在多线程环境中 POSIX 要求使用此函数（`sigprocmask` 在线程中的行为未定义）。

#### 前置条件

- `how` 为 `SIG_BLOCK` (0)、`SIG_UNBLOCK` (1) 或 `SIG_SETMASK` (2)
- `set` 可为 NULL（仅查询当前掩码时）
- `old` 可为 NULL（不关心旧掩码时）
- 若 `set != NULL`，`how` 必须有效

#### 后置条件

- Case 1（`set != NULL` 且 `how` 无效）：返回 `EINVAL`
- Case 2（有效调用）：通过 `SYS_rt_sigprocmask` 系统调用修改线程信号掩码
  - 若系统调用成功（返回 0）且 `old != NULL`：`*old` 包含之前的信号掩码，但内部信号位会被清除（用户看不到）
  - 清除的信号位包括 `SIGCANCEL` (33) 和 `SIGSYNCCALL` (34) 等内部信号：
    - 在 64 位系统上：`old->__bits[0] &= ~0x380000000ULL`（清除位 32-33）
    - 在 32 位系统上：`old->__bits[0] &= ~0x80000000UL` 和 `old->__bits[1] &= ~0x3UL`
  - 返回值为系统调用的错误码（0 = 成功）

#### 系统算法

```
pthread_sigmask(how, set, old):
  1. if set && (unsigned)how - SIG_BLOCK > 2U: return EINVAL
  2. ret = -__syscall(SYS_rt_sigprocmask, how, set, old, _NSIG/8)
  3. if !ret && old:
       if sizeof old->__bits[0] == 8:    // 64 位系统
         old->__bits[0] &= ~0x380000000ULL
       else:                              // 32 位系统
         old->__bits[0] &= ~0x80000000UL
         old->__bits[1] &= ~0x3UL
  4. return ret
```

#### 不变量

- 线程实际信号掩码包含内部信号（用于线程取消和同步调用），但 `old` 返回值中这些位被清除，避免用户误操作
- 信号处理函数中通过 `uc_sigmask` 获取运行上下文时，内部信号位也会被妥善屏蔽

#### 依赖

- `__syscall(SYS_rt_sigprocmask, ...)` — `rt_sigprocmask` 系统调用
- `SIG_BLOCK` / `SIG_UNBLOCK` / `SIG_SETMASK` — 信号操作常量（来自 `<signal.h>`）
- `EINVAL` — 错误码（来自 `<errno.h>`）
- `_NSIG` — 系统信号总数常量
- `sigset_t` — 信号集类型（来自 `<signal.h>` 和 `<bits/alltypes.h>`）
- 内部信号号：`SIGCANCEL` (33), `SIGSYNCCALL` (34)（定义于 `internal/pthread_impl.h`）
