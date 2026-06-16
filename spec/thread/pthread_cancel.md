# pthread_cancel.c 规约

> musl libc 的线程取消机制完整实现。包含取消信号处理、取消点包装、以及 `pthread_cancel()` 对外接口。使用 `SIGCANCEL` 实时信号实现异步取消路径，通过检查 `cancel` 标志和 `canceldisable` 状态在取消点触发同步取消。

---

## 依赖图

```
pthread_cancel (User API)
  ├─> init_cancellation()       — static (同文件)
  │     ├─> memset()            — see string/ (libc)
  │     └─> __libc_sigaction()  — see internal/pthread_impl.h (声明)
  ├─> a_store(&t->cancel, 1)    — see internal/atomic.h
  ├─> pthread_self()            — see pthread_self.c
  ├─> pthread_exit()            — see pthread_exit.c (libc)
  └─> pthread_kill(t, SIGCANCEL) — see pthread_kill.c

__cancel (hidden)
  ├─> __pthread_self()          — see internal/pthread_impl.h
  └─> pthread_exit()            — see pthread_exit.c

__syscall_cp_c (hidden)
  ├─> __pthread_self()          — see internal/pthread_impl.h
  ├─> __syscall()               — see internal/syscall.h
  └─> __syscall_cp_asm()        — hidden (同文件声明, asm 实现)

cancel_handler (static)
  ├─> __pthread_self()          — see internal/pthread_impl.h
  ├─> a_barrier()               — see internal/atomic.h
  ├─> _sigaddset()              — static (同文件)
  ├─> pthread_sigmask()         — see pthread_sigmask.c
  ├─> __cancel()                — hidden (同文件)
  └─> __syscall(SYS_tkill, ...) — see internal/syscall.h

__testcancel (hidden)
  ├─> __pthread_self()          — see internal/pthread_impl.h
  └─> __cancel()                — hidden (同文件)
```

---

## 内部辅助函数规约

### 1. _sigaddset (static)

```c
static void _sigaddset(sigset_t *set, int sig);
```

[Visibility]: Internal (不导出) — static 函数，仅本文件内使用

#### Intent

向信号集 `set` 中添加信号 `sig`。操作 `sigset_t` 内部的位掩码，将对应位设为 1。

#### 前置条件

- `set != NULL`
- `sig >= 1`

#### 后置条件

- `set->__bits[(sig-1) / (8*sizeof(set->__bits[0]))]` 中与 `sig` 对应的位被置为 1

#### 系统算法

```
_sigaddset(set, sig):
  1. s = sig - 1
  2. set->__bits[s / (8 * sizeof *set->__bits)] |= 1UL << (s & (8 * sizeof *set->__bits - 1))
```

---

### 2. cancel_handler (static)

```c
static void cancel_handler(int sig, siginfo_t *si, void *ctx);
```

[Visibility]: Internal (不导出) — static 函数，作为 `SIGCANCEL` 的信号处理函数注册

#### Intent

处理线程取消信号 `SIGCANCEL`。根据当前线程的取消状态决定立即退出（异步取消）或修改上下文 PC 跳转到取消代码（延迟取消）。

#### 前置条件

- 当前线程的 `cancel` 标志已被设置（由 `pthread_cancel` 设置）
- `ctx` 为有效的 `ucontext_t *`，包含被中断代码的寄存器状态

#### 后置条件

- 若 `self->cancel == 0` 或 `self->canceldisable == PTHREAD_CANCEL_DISABLE`：直接返回，不执行取消
- 将 `SIGCANCEL` 添加到 `uc_sigmask`（被中断上下文的信号掩码）
- Case 1 异步取消（`self->cancelasync != 0`）：恢复被中断信号掩码，调用 `__cancel()` 执行取消退出
- Case 2 延迟取消且 PC 在取消点范围内（`__cp_begin <= pc < __cp_end`）：修改 `uc_mcontext.MC_PC` 指向 `__cp_cancel`，使得信号处理返回后跳转到取消逻辑
- Case 3 延迟取消且 PC 不在取消点：调用 `SYS_tkill` 向当前线程重新发送 `SIGCANCEL`（被动等待下次取消点）

#### 系统算法

```
cancel_handler(sig, si, ctx):
  1. self = __pthread_self()
  2. uc = ctx; pc = uc->uc_mcontext.MC_PC
  3. a_barrier()
  4. if !self->cancel || self->canceldisable == PTHREAD_CANCEL_DISABLE: return
  5. _sigaddset(&uc->uc_sigmask, SIGCANCEL)
  6. if self->cancelasync:
       pthread_sigmask(SIG_SETMASK, &uc->uc_sigmask, 0)
       __cancel()
  7. if pc in [__cp_begin, __cp_end):   // 在取消点内部
       uc->uc_mcontext.MC_PC = (uintptr_t)__cp_cancel
       return                           // 信号返回后跳转到取消路径
  8. __syscall(SYS_tkill, self->tid, SIGCANCEL)  // 重新发送信号
```

---

### 3. init_cancellation (static)

```c
static void init_cancellation(void);
```

[Visibility]: Internal (不导出) — static 函数，延迟初始化取消信号处理

#### Intent

延迟注册 `SIGCANCEL` 的信号处理函数。使用 `SA_SIGINFO | SA_RESTART | SA_ONSTACK` 标志，并阻塞所有信号（掩码全部设为 1）以保证处理函数执行期间不被其他信号中断。

#### 前置条件

- 仅在首次调用 `pthread_cancel` 时执行一次（由 `static int init` 保护）

#### 后置条件

- `SIGCANCEL` (信号 33) 的信号处理函数被设置为 `cancel_handler`
- 后续任何线程收到 `SIGCANCEL` 时将执行 `cancel_handler`

---

## 内部 hidden 函数规约

### 4. __cancel (hidden)

```c
hidden long __cancel(void);
```

[Visibility]: Internal (不导出) — `hidden` 可见性，仅库内部使用

#### Intent

执行实际的线程取消操作。检查当前线程的取消状态：若未禁用取消或处于异步取消模式，则调用 `pthread_exit(PTHREAD_CANCELED)` 终止线程。否则将取消状态设为 DISABLE 并返回 `-ECANCELED`。

#### 前置条件

- 调用线程的 `cancel` 标志已被设置为非零值
- 由 `cancel_handler` 或 `__testcancel` 调用

#### 后置条件

- Case 1（`canceldisable == PTHREAD_CANCEL_ENABLE` 或 `cancelasync != 0`）：调用 `pthread_exit(PTHREAD_CANCELED)` 终止当前线程（不返回）
- Case 2（取消被临时禁用）：`self->canceldisable = PTHREAD_CANCEL_DISABLE`，返回 `-ECANCELED`

#### 系统算法

```
__cancel():
  1. self = __pthread_self()
  2. if self->canceldisable == PTHREAD_CANCEL_ENABLE || self->cancelasync:
       pthread_exit(PTHREAD_CANCELED)
  3. self->canceldisable = PTHREAD_CANCEL_DISABLE
  4. return -ECANCELED
```

---

### 5. __syscall_cp_c (hidden)

```c
hidden long __syscall_cp_c(syscall_arg_t nr,
    syscall_arg_t u, syscall_arg_t v, syscall_arg_t w,
    syscall_arg_t x, syscall_arg_t y, syscall_arg_t z);
```

[Visibility]: Internal (不导出) — `hidden` 可见性，作为取消点系统调用的 C 语言回退实现

#### Intent

包装可能阻塞的系统调用，使其成为"取消点"。若取消已被永久禁用或调用的是 `SYS_close`，则直接执行系统调用。否则通过 `__syscall_cp_asm` 执行可在取消时被中断的版本。

#### 前置条件

- 调用线程可能已收到取消请求（`self->cancel != 0`）
- 对于非 `SYS_close` 的系统调用，若被 `EINTR` 中断且取消已就绪，将执行 `__cancel()`

#### 后置条件

- Case 1（取消永久禁用 `PTHREAD_CANCEL_DISABLE` 或 `nr == SYS_close`）：直接执行 `__syscall(nr, ...)`，返回系统调用结果
- Case 2（正常取消点）：通过 `__syscall_cp_asm` 执行，若返回 `-EINTR` 且 `self->cancel && canceldisable != PTHREAD_CANCEL_DISABLE`，则调用 `__cancel()` 执行取消
- 返回系统调用结果

#### 系统算法

```
__syscall_cp_c(nr, u, v, w, x, y, z):
  1. self = __pthread_self()
  2. st = self->canceldisable
  3. if st && (st == PTHREAD_CANCEL_DISABLE || nr == SYS_close):
       return __syscall(nr, u, v, w, x, y, z)
  4. r = __syscall_cp_asm(&self->cancel, nr, u, v, w, x, y, z)
  5. if r == -EINTR && nr != SYS_close && self->cancel
       && self->canceldisable != PTHREAD_CANCEL_DISABLE:
       r = __cancel()
  6. return r
```

---

### 6. __testcancel (hidden)

```c
hidden void __testcancel(void);
```

[Visibility]: Internal (不导出) — `hidden` 可见性，被 `pthread_testcancel` 和取消点代码调用

#### Intent

显式取消点。检查当前线程的 `cancel` 标志，若已设置且未禁用取消，则立即执行取消退出（调用 `__cancel()`）。

#### 前置条件

- 调用线程的取消状态为启用 (`canceldisable != PTHREAD_CANCEL_DISABLE`)

#### 后置条件

- 若 `self->cancel && !self->canceldisable`：调用 `__cancel()` 执行取消（可能不返回）
- 否则直接返回

#### 系统算法

```
__testcancel():
  1. self = __pthread_self()
  2. if self->cancel && !self->canceldisable:
       __cancel()
```

---

## 对外导出函数规约

### 7. pthread_cancel

```c
int pthread_cancel(pthread_t t);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

向目标线程 `t` 发送取消请求。若目标为自身且处于异步取消 + 启用状态，立即退出。否则通过 `pthread_kill` 发送 `SIGCANCEL` 信号。

#### 前置条件

- `t` 为有效且存在的线程标识符
- 首次调用会触发 `init_cancellation()` 的一次性初始化

#### 后置条件

- `t->cancel = 1`（通过 `a_store` 原子写入，对所有线程可见）
- Case 1（`t == pthread_self()` 且 `canceldisable == PTHREAD_CANCEL_ENABLE` 且 `cancelasync`）：调用 `pthread_exit(PTHREAD_CANCELED)` 立即终止（不返回）
- Case 2（`t == pthread_self()` 但不满足立即退出条件）：返回 0，目标线程将在下一个取消点执行取消
- Case 3（`t != pthread_self()`）：发送 `SIGCANCEL` 信号给目标线程，返回 `pthread_kill` 的结果
- 返回值 0 表示取消请求已成功投递

#### 系统算法

```
pthread_cancel(t):
  1. if !init:
       init_cancellation()
       init = 1
  2. a_store(&t->cancel, 1)       // 设置取消标志
  3. if t == pthread_self():
       if t->canceldisable == PTHREAD_CANCEL_ENABLE && t->cancelasync:
         pthread_exit(PTHREAD_CANCELED)
       return 0
  4. return pthread_kill(t, SIGCANCEL)
```

#### 不变量

- 每个线程的 `cancel` 标志一旦被设置，不会自动清除（除非线程退出）
- `canceldisable` 为 `PTHREAD_CANCEL_DISABLE` 时取消请求会被暂缓，不会丢失

---

## 依赖

- `__pthread_self()` — 获取当前线程 `struct pthread *`（宏，展开为 `__get_tp()`），见 `internal/pthread_impl.h`
- `a_store()` / `a_barrier()` — 原子操作（见 `internal/atomic.h`）
- `__syscall()` — 系统调用
- `__syscall_cp_asm()` — 架构特定汇编实现的取消点系统调用包装
- `__libc_sigaction()` — libc 内部 sigaction
- `pthread_self()` — 当前线程的 `pthread_t`（见 `pthread_self.c`）
- `pthread_exit()` — 线程退出（见 `pthread_exit.c`）
- `pthread_sigmask()` — 信号掩码操作（见 `pthread_sigmask.c`）
- `pthread_kill()` — 向线程发送信号（见 `pthread_kill.c`）
- `memset()` — C 标准库（来自 `<string.h>`）
- `__cp_begin / __cp_end / __cp_cancel` — 取消点范围标记符号（由链接器脚本或汇编定义）
- `PTHREAD_CANCEL_ENABLE / DISABLE` / `PTHREAD_CANCELED` — 宏常量（见 `<pthread.h>`）
- `SIGCANCEL` / `SYS_tkill` / `SYS_close` — 信号号和系统调用号（见 `internal/pthread_impl.h` / `<sys/syscall.h>`）
