# synccall.c 规约

> musl libc 内部同步调用机制。向所有线程发送信号，令它们在信号处理器中执行指定回调，然后由调用线程串行化地让每个线程依次执行回调。用于需要跨所有线程同步执行的操作（如 TLS key 删除、setxid 等）。

---

## 依赖图

```
__synccall
  ├─> __block_app_sigs(&oldmask)                (外部 — 阻塞应用级信号)
  ├─> __tl_lock()                               (外部 — 线程列表锁, weak alias → dummy_0)
  ├─> __block_all_sigs(0)                       (外部 — 阻塞所有信号)
  ├─> pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &cs)  (外部 — POSIX)
  ├─> sem_init(&target_sem, 0, 0)               (外部 — POSIX semaphore)
  ├─> sem_init(&caller_sem, 0, 0)
  ├─> sem_init(&exit_sem, 0, 0)
  ├─> __pthread_self()                          (see pthread_impl.h — 当前线程)
  ├─> __syscall(SYS_gettid)                     (外部 — 获取内核线程 ID)
  ├─> __libc_sigaction(SIGSYNCCALL, &sa, 0)     (外部 — 信号处理)
  ├─> __syscall(SYS_tkill, td->tid, SIGSYNCCALL)(外部 — 发送信号)
  ├─> sem_wait(&caller_sem)                     (外部 — POSIX semaphore)
  ├─> sem_post(&target_sem)
  ├─> sem_destroy(...)                          (外部 — POSIX semaphore)
  ├─> __tl_unlock()
  └─> __restore_sigs(&oldmask)                  (外部 — 恢复信号掩码)

handler (signal handler, static)
  ├─> __pthread_self()
  ├─> sem_post(&caller_sem)
  ├─> sem_wait(&target_sem)
  ├─> callback(context)                         // 用户定义的回调
  ├─> sem_post(&caller_sem)
  ├─> sem_wait(&exit_sem)
  └─> sem_post(&caller_sem)
```

---

## 全局变量

### target_tid

```c
static int target_tid;
```

[Visibility]: Internal (不导出) — 文件作用域静态变量

当前被发信号的目标线程的 TID。

### callback / context

```c
static void (*callback)(void *), *context;
```

[Visibility]: Internal (不导出) — 文件作用域静态变量

保存 `__synccall` 调用者指定的回调函数指针和上下文参数。

### target_sem / caller_sem / exit_sem

```c
static sem_t target_sem, caller_sem, exit_sem;
```

[Visibility]: Internal (不导出) — 文件作用域静态变量

三个 POSIX 信号量，用于在调用线程和被信号触发的目标线程之间进行握手机制：
- `caller_sem`: 目标线程向调用者发信号（已抵达 / 已完成回调 / 已返回）
- `target_sem`: 调用者向目标线程发信号（可以开始执行回调）
- `exit_sem`: 调用者向目标线程发信号（可以退出信号处理器返回）

### 弱别名占位

```c
static void dummy_0(void) {}
weak_alias(dummy_0, __tl_lock);
weak_alias(dummy_0, __tl_unlock);
```

[Visibility]: Internal (不导出)

为 `__tl_lock` 和 `__tl_unlock` 提供默认空实现（单线程模式下无需锁操作）。多线程时由其他模块覆盖。

### dummy

```c
static void dummy(void *p) {}
```

[Visibility]: Internal (不导出) — 文件作用域静态变量

空回调占位函数。当 `__synccall` 无法向某线程发送信号时（`SYS_tkill` 失败），将 `callback` 替换为 `dummy` 以中止同步调用并释放已被捕获的线程。

---

## 函数规约

### 1. handler (static)

```c
static void handler(int sig);
```

[Visibility]: Internal (不导出) — 文件作用域静态信号处理器

#### Intent

在目标线程中执行的 `SIGSYNCCALL` 信号处理器。通过三个信号量 `caller_sem`、`target_sem`、`exit_sem` 与调用线程进行三步握手，保证回调的串行化执行和线程安全返回。

#### 前置条件

- 在接收到 `SIGSYNCCALL` 信号的线程上下文中执行
- `__synccall` 已设置 `callback` 和 `context`

#### 后置条件

- 若 `__pthread_self()->tid != target_tid`：直接返回（不是目标线程）
- 握手步骤：
  1. `sem_post(&caller_sem)` — 通知调用者已抵达
  2. `sem_wait(&target_sem)` — 等待调用者授权执行
  3. `callback(context)` — 执行用户定义的回调
  4. `sem_post(&caller_sem)` — 通知调用者回调已完成
  5. `sem_wait(&exit_sem)` — 等待调用者允许退出
  6. `sem_post(&caller_sem)` — 通知调用者正在返回（状态可销毁）
- `errno` 在返回前恢复为调用前的值

#### 系统算法

```
handler(sig):
  1. if (__pthread_self()->tid != target_tid) return    // 非目标线程
  2. old_errno = errno
  3. sem_post(&caller_sem)                              // step 1: 已抵达
  4. sem_wait(&target_sem)                              // step 2: 等待授权
  5. callback(context)                                   // step 3: 执行回调
  6. sem_post(&caller_sem)                              // step 4: 回调完成
  7. sem_wait(&exit_sem)                                // step 5: 等待退出信号
  8. sem_post(&caller_sem)                              // step 6: 正在返回
  9. errno = old_errno
```

---

### 2. __synccall

```c
void __synccall(void (*func)(void *), void *ctx);
```

[Visibility]: Internal (不导出) — 被 `libc.h` 和 `pthread_impl.h` 声明，仅 musl 内部使用

#### Intent

向进程中的所有其他线程发送 `SIGSYNCCALL` 信号，并串行化地令每个线程在信号处理器中执行 `func(ctx)`。调用者自身也执行一次 `func(ctx)`。用于需要所有线程一致性地执行某个操作（如删除 TLS key、更改 UID/GID 等）的场景。

设计保证了 AS-safety（异步信号安全）：通过两步信号阻塞（先阻塞应用级信号获取锁，再阻塞所有信号）避免死锁和重入。

#### 前置条件

- `func` 非空（若为空则退化为仅调用者执行）
- `ctx` 为调用者指定的上下文指针
- 调用时不持有 `__tl_lock` 或任何可能导致死锁的锁

#### 后置条件

- 单线程模式（`!libc.threads_minus_1` 或 `SYS_gettid != self->tid`）：
  - 直接跳转到 `single_threaded` 标签，仅在调用者中执行 `func(ctx)`
- 多线程模式：
  - 遍历线程列表，向除自己外的每个线程发送 `SIGSYNCCALL` 信号
  - 若某线程发送失败：将 `callback` 替换为 `dummy`，中止同步调用，释放已被捕获的线程
  - 对每个成功通知的线程：调用者依次通过 `sem_post(&target_sem)` 授权执行、`sem_wait(&caller_sem)` 等待完成
  - 调用者自身也执行 `func(ctx)`
  - 所有线程完成回调后：依次 `sem_post(&exit_sem)` 释放线程、`sem_wait(&caller_sem)` 确认线程已退出信号处理器
  - 销毁三个信号量，恢复信号掩码和取消状态

#### 系统算法

```
__synccall(func, ctx):
    // 阶段 0: AS-safe 信号阻塞（两步）
    __block_app_sigs(&oldmask)           // 先阻塞应用信号
    __tl_lock()                          // 获取线程列表锁
    __block_all_sigs(0)                  // 再阻塞全部信号
    pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &cs)
    
    // 阶段 1: 初始化
    sem_init(&target_sem, 0, 0)
    sem_init(&caller_sem, 0, 0)
    sem_init(&exit_sem, 0, 0)
    
    if (!libc.threads_minus_1 || __syscall(SYS_gettid) != self->tid)
        goto single_threaded
    
    callback = func; context = ctx
    
    // 阶段 2: 向所有其他线程发信号
    memset(&sa.sa_mask, -1, sizeof sa.sa_mask)   // 阻塞所有信号
    __libc_sigaction(SIGSYNCCALL, &sa, 0)
    
    for (td = self->next; td != self; td = td->next):
        target_tid = td->tid
        while (-__syscall(SYS_tkill, td->tid, SIGSYNCCALL) == EAGAIN)  // 重试
        if (r): callback = func = dummy; break     // 信号发送失败，中止
        sem_wait(&caller_sem)                      // 等待目标线程抵达
        count++
    target_tid = 0
    
    // 阶段 3: 串行化执行回调
    for (i = 0; i < count; i++):
        sem_post(&target_sem)                      // 授权一个线程执行
        sem_wait(&caller_sem)                      // 等待该线程完成
    
    sa.sa_handler = SIG_IGN
    __libc_sigaction(SIGSYNCCALL, &sa, 0)
    
single_threaded:
    func(ctx)                                      // 调用者自身执行
    
    // 阶段 4: 释放所有线程
    for (i = 0; i < count; i++) sem_post(&exit_sem)
    for (i = 0; i < count; i++) sem_wait(&caller_sem)
    
    // 阶段 5: 清理
    sem_destroy(&caller_sem)
    sem_destroy(&target_sem)
    sem_destroy(&exit_sem)
    pthread_setcancelstate(cs, 0)
    __tl_unlock()
    __restore_sigs(&oldmask)
```

#### 不变量

- 信号量 triple (`caller_sem`, `target_sem`, `exit_sem`) 始终保持握手协议的一致状态
- 在信号处理器 `handler` 激活期间，`SIGSYNCCALL` 被 `sa_mask` 阻塞，防止重入
- `callback` 和 `context` 仅在 `__synccall` 的执行期间有效，但在 `handler` 中被读取——由信号量握手保证同步
- 线程列表在 `__tl_lock` 保护下遍历，保证一致性

#### 依赖

- `__block_app_sigs()` / `__block_all_sigs()` / `__restore_sigs()` — 信号掩码操作（外部：musl 信号内部实现）
- `__tl_lock()` / `__tl_unlock()` — 线程列表锁（本文件 weak alias / 外部覆盖）
- `pthread_setcancelstate()` — POSIX 取消状态（外部：libc pthread 实现）
- `sem_init()` / `sem_wait()` / `sem_post()` / `sem_destroy()` — POSIX 信号量（外部：libc pthread 实现）
- `__pthread_self()` — 获取当前线程结构体（见 `pthread_impl.h`）
- `__libc_sigaction()` — 设置信号处理（外部：musl 信号实现）
- `__syscall(SYS_tkill, ...)` — 向线程发送信号（外部：Linux 系统调用）
- `libc.threads_minus_1` — 全局线程计数（见 `libc.h`）
- `SIGSYNCCALL` — 内部信号编号 34（见 `pthread_impl.h`）
