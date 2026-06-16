# synccall — Rust 接口归约

> rusl 内部同步调用机制。向所有线程发送信号，令它们在信号处理器中执行指定回调，然后由调用线程串行化地让每个线程依次执行回调。用于需要跨所有线程同步执行的操作（如 TLS key 删除、setxid 等）。Rust 实现中使用内部信号量和 futex 替代 POSIX semaphore。

## 原始 C 接口

```c
void __synccall(void (*func)(void *), void *ctx);
```

[Visibility]: Internal — 被 `libc.h` 和 `pthread_impl.h` 声明，仅 musl/rusl 内部使用

---

## Rust 外部 ABI 接口

```rust
pub extern "C" fn __synccall(
    func: Option<extern "C" fn(*mut core::ffi::c_void)>,
    ctx: *mut core::ffi::c_void,
);
```

---

## 依赖图

```
__synccall
  ├─> __block_app_sigs(&oldmask)               (外部 — 阻塞应用级信号)
  ├─> __tl_lock()                              (外部 — 线程列表锁, weak alias)
  ├─> __block_all_sigs()                       (外部 — 阻塞所有信号)
  ├─> pthread_setcancelstate(PTHREAD_CANCEL_DISABLE) (外部 — POSIX)
  ├─> __pthread_self()                         (外部 — 当前线程)
  ├─> linux syscall: SYS_gettid                (外部 — 获取内核线程 ID)
  ├─> __libc_sigaction(SIGSYNCCALL, ...)       (外部 — 信号处理)
  ├─> linux syscall: SYS_tkill                 (外部 — 发送信号)
  ├─> 内部信号量: Semaphore (target_sem, caller_sem, exit_sem)
  ├─> __tl_unlock()                            (外部)
  └─> __restore_sigs(&oldmask)                 (外部 — 恢复信号掩码)

handler (信号处理器, 内部 static)
  ├─> __pthread_self()
  ├─> 内部信号量 post/wait 握手
  └─> callback(context)                        // 用户定义的回调
```

---

## 内部数据结构（不对外导出）

### Semaphore

```rust
// 基于 AtomicI32 + futex 的内部信号量，替代 POSIX sem_t
struct Semaphore {
    value: core::sync::atomic::AtomicI32,
}

impl Semaphore {
    const fn new(init: i32) -> Self;
    fn post(&self);             // 递增信号量并唤醒等待者 (futex FUTEX_WAKE)
    fn wait(&self);             // 递减信号量，若 value <= 0 则 futex 等待
    fn destroy(&self);          // 销毁信号量 (no-op for futex-based)
}
```

### synccall 静态状态

```rust
// 文件作用域静态变量，不对外导出
static mut TARGET_TID: i32 = 0;                 // 当前被发信号的目标线程 TID
static mut CALLBACK: Option<extern "C" fn(*mut c_void)> = None;  // 用户回调
static mut CONTEXT: *mut c_void = null_mut();   // 用户回调上下文

static TARGET_SEM: Semaphore = Semaphore::new(0);
static CALLER_SEM: Semaphore = Semaphore::new(0);
static EXIT_SEM: Semaphore = Semaphore::new(0);
```

---

## 函数规约

### 1. handler (信号处理器, 内部 static)

```rust
// 文件作用域静态信号处理器，不对外导出
extern "C" fn handler(sig: core::ffi::c_int);
```

#### Intent

在目标线程中执行的 `SIGSYNCCALL` 信号处理器。通过三个信号量 `caller_sem`、`target_sem`、`exit_sem` 与调用线程进行三步握手，保证回调的串行化执行和线程安全返回。

#### 系统算法

```
handler(sig):
  1. if (__pthread_self().tid != TARGET_TID) return   // 非目标线程
  2. old_errno = errno
  3. CALLER_SEM.post()                                 // step 1: 已抵达
  4. TARGET_SEM.wait()                                 // step 2: 等待授权
  5. if let Some(f) = CALLBACK { f(CONTEXT) }          // step 3: 执行回调
  6. CALLER_SEM.post()                                 // step 4: 回调完成
  7. EXIT_SEM.wait()                                   // step 5: 等待退出信号
  8. CALLER_SEM.post()                                 // step 6: 正在返回
  9. errno = old_errno
```

---

### 2. __synccall

```rust
pub extern "C" fn __synccall(
    func: Option<extern "C" fn(*mut core::ffi::c_void)>,
    ctx: *mut core::ffi::c_void,
);
```

#### Intent

向进程中的所有其他线程发送 `SIGSYNCCALL` 信号，并串行化地令每个线程在信号处理器中执行 `func(ctx)`。调用者自身也执行一次 `func(ctx)`。用于需要所有线程一致性地执行某个操作（如删除 TLS key、更改 UID/GID 等）的场景。

设计保证了 AS-safety（异步信号安全）：通过两步信号阻塞（先阻塞应用级信号获取锁，再阻塞所有信号）避免死锁和重入。

#### 前置条件

- `func` 可为 `None`（仅调用者执行）
- `ctx` 为调用者指定的上下文指针
- 调用时不持有 `__tl_lock` 或任何可能导致死锁的锁

#### 后置条件

- 单线程模式：仅在调用者中执行 `func(ctx)`
- 多线程模式：
  - 遍历线程列表，向除自己外的每个线程发送 `SIGSYNCCALL` 信号
  - 若某线程发送失败：替换回调为空操作，中止同步调用，释放已被捕获的线程
  - 对每个成功通知的线程：调用者依次授权执行、等待完成
  - 调用者自身也执行 `func(ctx)`
  - 所有线程完成回调后：释放线程、确认退出、销毁信号量
  - 恢复信号掩码和取消状态

#### 系统算法

```
__synccall(func, ctx):
    // 阶段 0: AS-safe 信号阻塞（两步）
    __block_app_sigs(&oldmask)
    __tl_lock()
    __block_all_sigs()
    pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &cs)

    // 阶段 1: 初始化
    TARGET_SEM = Semaphore::new(0)
    CALLER_SEM = Semaphore::new(0)
    EXIT_SEM = Semaphore::new(0)

    if (!libc.threads_minus_1 || SYS_gettid != self.tid)
        goto single_threaded

    CALLBACK = func; CONTEXT = ctx

    // 阶段 2: 向所有其他线程发信号
    __libc_sigaction(SIGSYNCCALL, &sa, 0)  // sa_mask 阻塞所有信号

    for td in thread_list (skip self):
        TARGET_TID = td.tid
        while (SYS_tkill(td.tid, SIGSYNCCALL) fails with EAGAIN): retry
        if (r != 0): CALLBACK = None; func = None; break
        CALLER_SEM.wait()                  // 等待目标线程抵达
        count += 1
    TARGET_TID = 0

    // 阶段 3: 串行化执行回调
    for i in 0..count:
        TARGET_SEM.post()                  // 授权一个线程执行
        CALLER_SEM.wait()                  // 等待该线程完成

    sa.sa_handler = SIG_IGN
    __libc_sigaction(SIGSYNCCALL, &sa, 0)

single_threaded:
    if let Some(f) = func { f(ctx) }       // 调用者自身执行

    // 阶段 4: 释放所有线程
    for i in 0..count: EXIT_SEM.post()
    for i in 0..count: CALLER_SEM.wait()

    // 阶段 5: 清理
    CALLER_SEM.destroy()
    TARGET_SEM.destroy()
    EXIT_SEM.destroy()
    pthread_setcancelstate(cs, 0)
    __tl_unlock()
    __restore_sigs(&oldmask)
```

#### 不变量

- 信号量 triple 始终保持握手协议的一致状态
- 在信号处理器激活期间，`SIGSYNCCALL` 被阻塞，防止重入
- `CALLBACK` 和 `CONTEXT` 仅在 `__synccall` 执行期间有效，但由信号量握手保证同步
- 线程列表在 `__tl_lock` 保护下遍历

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 内部信号量实现（基于 futex）
struct FutexSemaphore {
    value: core::sync::atomic::AtomicI32,
}

impl FutexSemaphore {
    pub(crate) const fn new(init: i32) -> Self { /* ... */ }
    pub(crate) fn post(&self) { /* futex FUTEX_WAKE */ }
    pub(crate) fn wait(&self) { /* CAS + futex FUTEX_WAIT */ }
    pub(crate) fn destroy(&self) { /* no-op */ }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::sync::atomic::AtomicI32          // 依赖1: 原子类型
  __block_app_sigs() / __block_all_sigs() / __restore_sigs()  // 依赖2: 信号掩码
  __tl_lock() / __tl_unlock()            // 依赖3: 线程列表锁
  __pthread_self()                        // 依赖4: 当前线程结构体
  __libc_sigaction()                      // 依赖5: 信号处理设置
  linux syscall: SYS_gettid / SYS_tkill  // 依赖6: 系统调用
  libc.threads_minus_1                    // 依赖7: 全局线程计数
  SIGSYNCCALL                             // 依赖8: 内部信号编号

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __synccall(func: Option<extern "C" fn(*mut c_void)>, ctx: *mut c_void);
                                         // 本模块保证对外提供与 C ABI 兼容的 __synccall 符号
Internal Interface:
  struct FutexSemaphore { value: AtomicI32 }
  impl FutexSemaphore { pub(crate) fn new(i32), post(), wait(), destroy() }
                                         // 内部 futex 信号量，供 crate 内部使用
