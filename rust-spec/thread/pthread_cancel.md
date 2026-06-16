# pthread_cancel — Rust 接口归约

## 原始 C 接口
```c
// === 对外导出函数 ===

// 对外导出（POSIX），向目标线程发送取消请求
int pthread_cancel(pthread_t t);

// === 内部 hidden 函数 ===

// 取消检查核心，被 __testcancel 调用
hidden long __cancel(void);

// 取消点系统调用包装的 C 回退实现
hidden long __syscall_cp_c(syscall_arg_t nr, syscall_arg_t u, syscall_arg_t v,
    syscall_arg_t w, syscall_arg_t x, syscall_arg_t y, syscall_arg_t z);

// 取消检查入口，被 pthread_testcancel 和取消点代码调用
hidden void __testcancel(void);

// === 内部 static 函数 ===

// 信号集位操作辅助
static void _sigaddset(sigset_t *set, int sig);

// SIGCANCEL 信号处理函数
static void cancel_handler(int sig, siginfo_t *si, void *ctx);

// 延迟注册取消信号处理
static void init_cancellation(void);
```

---

## Rust 外部 ABI 接口

```rust
// ========== 对外导出函数 ==========

// POSIX 用户接口
extern "C" fn pthread_cancel(t: pthread_t) -> core::ffi::c_int;

// ========== 内部导出函数（hidden，被其他模块引用）==========

// 取消检查核心
extern "C" fn __cancel() -> core::ffi::c_long;

// 取消检查入口
extern "C" fn __testcancel();

// 取消点系统调用包装
extern "C" fn __syscall_cp_c(nr: syscall_arg_t, u: syscall_arg_t, v: syscall_arg_t,
    w: syscall_arg_t, x: syscall_arg_t, y: syscall_arg_t, z: syscall_arg_t) -> core::ffi::c_long;
```

---

## 意图
实现完整的线程取消机制，包括：
1. `pthread_cancel`：向目标线程发送取消请求
2. `__testcancel` / `__cancel`：在取消点检查并执行取消
3. `__syscall_cp_c`：包装可能阻塞的系统调用，使其成为取消点
4. `cancel_handler`：处理 `SIGCANCEL` 信号，实现异步取消和延迟取消的 PC 修改
5. `init_cancellation`：延迟注册取消信号处理函数

## 前置条件
- 对于 `pthread_cancel(t)`：`t` 为有效且存在的线程标识符
- 首次调用 `pthread_cancel` 会触发 `init_cancellation()` 的一次性初始化
- `cancel_handler` 被注册为 `SIGCANCEL` 的信号处理函数

## 后置条件

**pthread_cancel:**
- `t->cancel = 1`（原子写入，对所有线程可见）
- Case 1（`t == pthread_self()` 且异步取消）：调用 `pthread_exit(PTHREAD_CANCELED)` 立即终止
- Case 2（`t == pthread_self()` 但不满足立即退出）：返回 0
- Case 3（`t != pthread_self()`）：发送 `SIGCANCEL` 信号，返回 `pthread_kill` 的结果

**__cancel:**
- Case 1（取消启用或异步）：调用 `pthread_exit(PTHREAD_CANCELED)` 终止线程（不返回）
- Case 2（取消被临时禁用）：`self->canceldisable = PTHREAD_CANCEL_DISABLE`，返回 `-ECANCELED`

**cancel_handler:**
- 根据取消状态决定：直接返回 / 调用 `__cancel()` 退出 / 修改 PC 跳转到取消代码 / 重新发送信号

## 不变量
- 每个线程的 `cancel` 标志一旦被设置，不会自动清除（除非线程退出）
- `canceldisable` 为 `PTHREAD_CANCEL_DISABLE` 时取消请求会被暂缓，不会丢失

## 算法

```rust
use core::sync::atomic::{AtomicI32, Ordering};

// ========== 内部辅助函数 ==========

// _sigaddset 的 Rust 等效实现
fn sigaddset(set: &mut Sigset, sig: u32) {
    let s = sig - 1;
    let word_size = 8 * core::mem::size_of::<usize>();
    set.bits[(s as usize) / word_size] |= 1usize << (s as usize % word_size);
}

// init_cancellation — 延迟初始化
// 使用 AtomicBool 保护一次性初始化
static INIT: AtomicBool = AtomicBool::new(false);

fn init_cancellation() {
    if INIT.swap(true, Ordering::Acquire) {
        return;  // 已被初始化
    }
    // 注册 SIGCANCEL 信号处理函数为 cancel_handler
    // 使用 SA_SIGINFO | SA_RESTART | SA_ONSTACK 标志
    // 阻塞所有信号以保证处理函数执行期间不被其他信号中断
    // 通过内部 sigaction 接口注册
    register_signal_handler(SIGCANCEL, cancel_handler, SA_SIGINFO | SA_RESTART | SA_ONSTACK);
}

// ========== pthread_cancel — 对外导出 ==========

pub extern "C" fn pthread_cancel(t: pthread_t) -> core::ffi::c_int {
    // 延迟初始化
    init_cancellation();

    // 设置 cancel 标志（原子写入）
    let cancel_ptr = &t.cancel as *const _ as *const AtomicI32;
    unsafe { &*cancel_ptr }.store(1, Ordering::Release);

    // 检查是否取消自身
    let myself = pthread_self();
    if t == myself {
        // 若自身为异步取消且启用状态，立即退出
        if t.canceldisable == PTHREAD_CANCEL_ENABLE && t.cancelasync != 0 {
            pthread_exit(PTHREAD_CANCELED);
        }
        return 0;
    }

    // 向目标线程发送 SIGCANCEL
    pthread_kill(t, SIGCANCEL)
}

// ========== __cancel — 执行实际的取消退出 ==========

pub extern "C" fn __cancel() -> core::ffi::c_long {
    let current = current_thread();
    if current.canceldisable == PTHREAD_CANCEL_ENABLE || current.cancelasync != 0 {
        // 执行取消退出（不会返回）
        pthread_exit(PTHREAD_CANCELED);
    }
    // 取消被临时禁用，返回 ECANCELED
    current.canceldisable = PTHREAD_CANCEL_DISABLE;
    -(ECANCELED as core::ffi::c_long)
}

// ========== __testcancel — 取消检查入口 ==========

pub extern "C" fn __testcancel() {
    let current = current_thread();
    if current.cancel != 0 && current.canceldisable == PTHREAD_CANCEL_ENABLE {
        __cancel();
    }
}

// ========== __syscall_cp_c — 取消点系统调用包装 ==========

pub extern "C" fn __syscall_cp_c(nr: syscall_arg_t, u: syscall_arg_t, v: syscall_arg_t,
    w: syscall_arg_t, x: syscall_arg_t, y: syscall_arg_t, z: syscall_arg_t) -> core::ffi::c_long {
    let current = current_thread();
    let st = current.canceldisable;

    // 若取消永久禁用或调用 SYS_close，直接执行系统调用
    if st != 0 && (st == PTHREAD_CANCEL_DISABLE || nr == SYS_close as syscall_arg_t) {
        return __syscall(nr, u, v, w, x, y, z);
    }

    // 通过 __syscall_cp_asm 执行（可在取消时被中断的版本）
    let r = __syscall_cp_asm(&current.cancel, nr, u, v, w, x, y, z);

    // 若被 EINTR 中断且取消已就绪
    if r == -(EINTR as core::ffi::c_long) && nr != SYS_close as syscall_arg_t
        && current.cancel != 0 && current.canceldisable != PTHREAD_CANCEL_DISABLE
    {
        return __cancel();
    }
    r
}

// ========== cancel_handler — SIGCANCEL 信号处理 ==========

fn cancel_handler(sig: i32, si: &siginfo_t, ctx: &mut ucontext_t) {
    let current = current_thread();
    let pc = ctx.uc_mcontext.pc;  // 被中断代码的 PC

    // 内存屏障确保观察到最新的 cancel 标志
    fence(Ordering::Acquire);

    if current.cancel == 0 || current.canceldisable == PTHREAD_CANCEL_DISABLE {
        return;  // 不执行取消
    }

    // 将 SIGCANCEL 添加到被中断上下文的信号掩码
    sigaddset(&mut ctx.uc_sigmask, SIGCANCEL);

    if current.cancelasync != 0 {
        // 异步取消：恢复信号掩码并立即退出
        pthread_sigmask(SIG_SETMASK, &ctx.uc_sigmask, core::ptr::null_mut());
        __cancel();
        // 不会到达此处
    }

    // 延迟取消：检查 PC 是否在取消点范围内
    if pc >= __cp_begin && pc < __cp_end {
        // PC 指向取消点内部，修改 PC 跳转到取消路径
        ctx.uc_mcontext.pc = __cp_cancel as usize;
        return;  // 信号返回后执行取消逻辑
    }

    // PC 不在取消点，重新发送信号（等待下次取消点）
    __syscall(SYS_tkill, current.tid, SIGCANCEL);
}
```

对 C 调用者：
1. `extern "C" fn pthread_cancel(t: pthread_t) -> c_int` 向目标线程发送取消请求
2. 内部通过原子操作设置 `cancel` 标志，并通过 `SIGCANCEL` 信号或直接退出实现取消
3. 返回值 0 表示取消请求已成功投递

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
use core::sync::atomic::{AtomicBool, Ordering};

// 安全的 Rust 线程取消上下文
pub(crate) struct CancelContext {
    cancel: AtomicBool,        // 取消请求标志
    canceldisable: AtomicI32,  // 取消禁用状态 (0=ENABLE, 1=DISABLE)
    cancelasync: AtomicI32,    // 取消类型 (0=DEFERRED, 1=ASYNCHRONOUS)
}

impl CancelContext {
    pub(crate) fn testcancel(&self) {
        if self.cancel.load(Ordering::Relaxed)
            && self.canceldisable.load(Ordering::Relaxed) == PTHREAD_CANCEL_ENABLE
        {
            self.do_cancel();
        }
    }

    fn do_cancel(&self) -> ! {
        // 执行取消退出（通过 pthread_exit）
        // 此函数不返回
    }
}
```

---

/* Rely */
[RELY]
Predefined Types/Functions:
  core::sync::atomic::{AtomicI32, AtomicBool, Ordering, fence}  // 依赖1: 原子操作
  __pthread_self() / current_thread()    // 依赖2: 获取当前线程指针（TLS）
  pthread_self()                         // 依赖3: 获取当前 pthread_t
  pthread_exit()                         // 依赖4: 线程退出
  pthread_kill()                         // 依赖5: 向线程发送信号
  pthread_sigmask()                      // 依赖6: 信号掩码操作
  __syscall()                            // 依赖7: 系统调用
  __syscall_cp_asm()                     // 依赖8: 架构特定取消点系统调用（汇编实现）
  __libc_sigaction()                     // 依赖9: libc 内部 sigaction
Predefined Constants:
  PTHREAD_CANCEL_ENABLE (0)             // 依赖10
  PTHREAD_CANCEL_DISABLE (1)            // 依赖11
  PTHREAD_CANCEL_DEFERRED (0)           // 依赖12
  PTHREAD_CANCEL_ASYNCHRONOUS (1)       // 依赖13
  PTHREAD_CANCELED                       // 依赖14: 取消退出状态值
  SIGCANCEL                              // 依赖15: 取消信号号 (33)
  SIG_SETMASK                            // 依赖16
  EINTR / ECANCELED / EINVAL            // 依赖17: errno 值
  SYS_tkill / SYS_close                  // 依赖18: 系统调用号
  SA_SIGINFO / SA_RESTART / SA_ONSTACK  // 依赖19: sigaction 标志
Predefined Symbols (链接器/汇编定义):
  __cp_begin / __cp_end / __cp_cancel   // 依赖20: 取消点范围标记符号

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_cancel(t: pthread_t) -> core::ffi::c_int;
  extern "C" fn __cancel() -> core::ffi::c_long;
  extern "C" fn __testcancel();
  extern "C" fn __syscall_cp_c(nr: syscall_arg_t, ...) -> core::ffi::c_long;
                                    // 本模块保证对外提供与 C ABI 兼容的上述符号
Internal Interface:
  pub(crate) struct CancelContext;
  pub(crate) fn testcancel(ctx: &CancelContext);
                                    // 安全包装，供 crate 内部使用
