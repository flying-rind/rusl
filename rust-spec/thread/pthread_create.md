# pthread_create — Rust 接口归约

> rusl 线程创建与退出的核心实现。包含线程列表锁、线程退出、清理处理栈、线程启动桩和线程创建。

---

## 原始 C 对外导出接口

```c
// 用户可见符号 (weak_alias)
int pthread_create(pthread_t *restrict res, const pthread_attr_t *restrict attrp,
                   void *(*entry)(void *), void *restrict arg);
_Noreturn void pthread_exit(void *result);

// musl 内部跨模块调用符号 (hidden / non-static)
int __pthread_create(pthread_t *restrict res, const pthread_attr_t *restrict attrp,
                     void *(*entry)(void *), void *restrict arg);
_Noreturn void __pthread_exit(void *result);
void __tl_lock(void);
void __tl_unlock(void);
void __tl_sync(pthread_t td);
void __do_cleanup_push(struct __ptcb *cb);
void __do_cleanup_pop(struct __ptcb *cb);
```

---

## Rust 外部 ABI 接口

```rust
// 用户可见符号 — 与 C ABI 兼容
extern "C" fn pthread_create(
    res: *mut *mut core::ffi::c_void,
    attrp: *const core::ffi::c_void,
    entry: extern "C" fn(*mut core::ffi::c_void) -> *mut core::ffi::c_void,
    arg: *mut core::ffi::c_void,
) -> core::ffi::c_int;

extern "C" fn pthread_exit(result: *mut core::ffi::c_void) -> !;

// musl 内部跨模块调用符号 — 与 C ABI 兼容
extern "C" fn __pthread_create(
    res: *mut *mut core::ffi::c_void,
    attrp: *const core::ffi::c_void,
    entry: extern "C" fn(*mut core::ffi::c_void) -> *mut core::ffi::c_void,
    arg: *mut core::ffi::c_void,
) -> core::ffi::c_int;

extern "C" fn __pthread_exit(result: *mut core::ffi::c_void) -> !;

extern "C" fn __tl_lock();
extern "C" fn __tl_unlock();
extern "C" fn __tl_sync(td: *mut core::ffi::c_void);

extern "C" fn __do_cleanup_push(cb: *mut Ptcb);
extern "C" fn __do_cleanup_pop(cb: *mut Ptcb);
```

---

## 内部类型定义（Rust 重新设计）

### StartArgs

原 C 的 `struct start_args` 位于新线程栈上，传递线程入口信息。在 Rust 中重新设计为 `repr(C)` 结构体，确保栈上布局可预测：

```rust
#[repr(C)]
struct StartArgs {
    /// 线程入口函数
    start_func: extern "C" fn(*mut core::ffi::c_void) -> *mut core::ffi::c_void,
    /// 线程入口参数
    start_arg: *mut core::ffi::c_void,
    /// 调度控制标志: 0=无需调度, 1=父线程准备, 2=子线程确认, 3=调度失败
    control: core::ffi::c_int,
    /// 信号掩码副本
    sig_mask: [core::ffi::c_ulong; SIGSET_SZ],
}
```

### Ptcb（清理处理控制块）

```rust
#[repr(C)]
struct Ptcb {
    /// 清理处理函数
    __f: extern "C" fn(*mut core::ffi::c_void),
    /// 清理处理参数
    __x: *mut core::ffi::c_void,
    /// 链表下一节点
    __next: *mut Ptcb,
}
```

### DetachState（线程分离状态枚举）

```rust
#[repr(i32)]
enum DetachState {
    Exited = 0,
    Exiting = 1,
    Joinable = 2,
    Detached = 3,
}
```

---

## 意图

### pthread_create / __pthread_create
创建新的 POSIX 线程。负责：首次线程化初始化、属性解析、栈/TLS/TSD 内存分配与映射、`struct pthread` 初始化、通过 `clone` 系统调用创建内核线程、可选的调度属性设置、将新线程插入全局线程列表。

### pthread_exit / __pthread_exit
终止当前线程，执行完整的退出清理流程：运行取消清理处理函数、TSDs 析构函数、处理 robust mutex 列表、从线程列表移除自身、对分离线程自动释放资源。`_Noreturn` 属性在 Rust 中对应 `-> !`（永不返回）。

### __tl_lock / __tl_unlock
获取/释放线程列表锁，支持递归锁定。Rust 内部可使用 `Mutex` 风格的 RAII 守卫模式重新设计。

### __tl_sync
等待线程列表锁变为空闲，用于 `pthread_join` 路径确保退出线程已完全从列表移除。

### __do_cleanup_push / __do_cleanup_pop
清理处理栈操作（由 `pthread_cleanup_push`/`pthread_cleanup_pop` 宏调用）。Rust 内部可使用 `Vec<CleanupHandler>` 替代裸链表。

---

## 前置条件

### pthread_create
- `res != null()`，指向可写入 `pthread_t` 的内存
- `entry` 不为 null，合法的线程入口函数
- `attrp` 可为 null（使用默认属性）或指向有效的 `pthread_attr_t`
- `libc.can_do_threads` 须为 true

### pthread_exit
- 调用者为退出的线程自身
- `result` 为线程返回值（可为 `PTHREAD_CANCELED` 等特殊值）

### __tl_lock
- 调用者须已阻塞应用层信号（AS-safety）
- `__pthread_self()` 返回有效的线程结构体

### __tl_unlock
- 调用者须持有线程列表锁

### __do_cleanup_push / __do_cleanup_pop
- `cb` 不为 null

---

## 后置条件

### pthread_create
- Case 1 成功：新线程被创建并开始执行 `entry(arg)`，`*res` 指向新线程的 `struct pthread`，返回 0
- Case 2 失败（系统不支持线程）：返回 `ENOSYS`
- Case 3 失败（内存分配/映射失败）：返回 `EAGAIN`
- Case 4 失败（clone 失败）：返回 `EAGAIN`

### pthread_exit
- 线程终止，不返回
- Joinable 线程：`detach_state` 设为 `DT_EXITED`，futex 唤醒 joiner
- Detached 线程：`__unmapself` 释放栈映射
- 最后一个线程（`self.next == self`）：调用 `exit(0)` 终止进程

### __tl_lock
- 线程独占持有线程列表锁，支持递归获取

### __tl_unlock
- 递归计数器递减或锁释放；若有等待者则唤醒

---

## 不变量

- **退出原子性**：`detach_state` 的 CAS 操作是 `pthread_detach` 和 `__pthread_exit` 之间的竞赛仲裁点
- **TID 安全性**：持有 `killlock` 期间禁止其他线程通过 TID 访问正在退出的线程
- **信号安全性**：阻塞应用信号后线程列表锁为 AS-safe
- **线程列表**：始终为双向循环链表
- **`libc.need_locks`**：当 `threads_minus_1 > 0` 时设为 1，为 0 时设为 -1 或 0

---

## 算法（内部实现策略）

### pthread_create 内部实现

Rust 内部可使用更安全的方式组织各阶段：

```
pthread_create_impl(res, attrp, entry, arg):
  // === 阶段 1: 首次线程化初始化 ===
  如果 !libc.threaded:
    遍历已打开 FILE 列表初始化文件锁
    解除阻塞 SIGCANCEL
    初始化 membarrier
    设置 libc.threaded = 1

  // === 阶段 2: 解析线程属性 ===
  如果 attrp 为 null: 使用默认栈大小和 guard 大小

  // === 阶段 3: 计算栈/TLS/TSD 布局 ===
  计算所需内存大小，确定 guard 页、栈、TLS、TSD 的位置

  // === 阶段 4: 分配和映射内存 ===
  使用 mmap 分配带 guard 页的栈空间
  Rust 内部可将 mmap 结果封装为 RAII 类型

  // === 阶段 5: 初始化 struct pthread ===
  填充 new: map_base, map_size, stack, stack_size, guard_size,
            self, tsd, locale, detach_state, robust_list, canary, sysinfo

  // === 阶段 6: 设置 StartArgs ===
  在子线程栈顶放置 StartArgs { start_func, start_arg, control, sig_mask }

  // === 阶段 7: 阻塞信号并配置掩码 ===
  确保 SIGCANCEL 在新线程中被解除阻塞

  // === 阶段 8: clone 系统调用 ===
  __clone(start, stack, flags, args, &new.tid, TP_ADJ(new), &__thread_list_lock)

  // === 阶段 9: 调度设置同步 ===
  如果 attr._a_sched: 通过 control 字段与子线程同步调度设置

  // === 阶段 10: 插入线程列表 ===
  将 new 插入双向循环链表 self <-> new <-> self.next

  // === 失败清理 ===
  munmap 释放映射，返回错误码
```

### pthread_exit 内部实现

Rust 内部可将清理阶段组织为清晰的分阶段流程：

```
__pthread_exit_impl(result):
  // === 阶段 1: 禁止取消 ===
  // === 阶段 2: 运行清理处理栈（LIFO 顺序）===
  // === 阶段 3: 运行 TSD 析构函数 ===
  // === 阶段 4: 阻塞应用信号 ===
  // === 阶段 5: CAS 设置退出状态（仲裁点）===
  // === 阶段 6: 获取 killlock 和线程列表锁 ===
  // === 阶段 7: 最后一个线程检测 ===
  // === 阶段 8: 清除 TID ===
  // === 阶段 9: 处理 robust mutex 列表 ===
  // === 阶段 10: stdio 和动态链接清理 ===
  // === 阶段 11: 从线程列表移除自身 ===
  // === 阶段 12: Detached: __unmapself / Joinable: futex wake joiner ===
  // === 阶段 13: 循环 SYS_exit 等待内核终止 ===
```

### start / start_c11（内部线程入口桩）

原 C 的 `static` 函数，在 Rust 中可重新设计为普通私有函数：

```rust
// 内部函数，非 extern "C"
unsafe fn thread_start(args: *mut StartArgs) -> ! {
    // 调度同步等待
    // 恢复信号掩码
    // 调用用户入口函数
    // 调用 __pthread_exit（永不返回）
}

// C11 线程入口桩
unsafe fn thread_start_c11(args: *mut StartArgs) -> ! {
    // 与 thread_start 类似，但返回值类型为 int
}
```

### __tl_lock / __tl_unlock（Rust 内部可用 Mutex 替代）

Rust 内部可使用 `core::sync::atomic` 原子操作替代 `a_cas`，配合 futex 实现自旋锁。对外 `extern "C"` 接口保持不变。

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 线程列表锁 RAII 守卫（内部使用）
pub(crate) struct TlLockGuard {
    // 持有锁标记
}

impl TlLockGuard {
    pub(crate) fn acquire() -> Self {
        // 内部调用 __tl_lock 的安全封装
    }
}

impl Drop for TlLockGuard {
    fn drop(&mut self) {
        // 内部调用 __tl_unlock 的安全封装
    }
}

// 清理处理栈管理（内部使用，替代裸链表）
pub(crate) struct CleanupStack {
    // 内部使用 Vec 或小型栈管理清理处理函数
}

// 线程创建参数（内部使用，替代裸 StartArgs）
pub(crate) struct ThreadCreateParams {
    pub(crate) start_func: extern "C" fn(*mut core::ffi::c_void) -> *mut core::ffi::c_void,
    pub(crate) arg: *mut core::ffi::c_void,
    pub(crate) sched_control: bool,
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::sync::atomic::{AtomicI32, AtomicUsize, Ordering}     // 依赖1: 原子操作（替代 a_cas/a_swap/a_store）
  core::ffi::{c_int, c_void, c_ulong}                         // 依赖2: C ABI 基本类型
  core::ptr                                                     // 依赖3: 裸指针操作
  __clone          (外部模块 clone)             // clone 系统调用封装
  __copy_tls       (外部模块 tls)               // TLS 数据复制
  __unmapself      (外部模块 thread)            // 分离线程自我销毁
  __syscall        (外部模块 syscall)           // 系统调用
  __mmap / __munmap / __mprotect (外部模块 mmap) // 内存映射管理
  __vm_wait / __vm_lock / __vm_unlock (外部模块 mmap) // 虚拟内存锁
  __wait / __wake  (外部模块 futex)             // futex 等待/唤醒
  __block_app_sigs / __block_all_sigs / __restore_sigs (外部模块 thread) // 信号掩码管理
  __pthread_self   (外部模块 thread)            // 获取当前线程结构体
  __ofl_lock / __ofl_unlock (外部模块 stdio)     // 打开文件列表锁
  memcpy / memset  (外部模块 string)            // 内存操作
  exit             (外部模块 stdlib)            // 进程退出
  libc 全局结构体  (内部 libc)                  // thread_minus_1, need_locks, threaded, can_do_threads
  __default_stacksize / __default_guardsize (外部模块 thread) // 默认栈/guard 大小
  __pthread_tsd_size / __pthread_tsd_main (weak / 外部模块)   // TSD 区域

Predefined Macros:
  SIGSET_SZ (_NSIG/8/sizeof(long))              // 信号掩码数组大小

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_create(res: *mut *mut core::ffi::c_void, attrp: *const core::ffi::c_void,
                               entry: extern "C" fn(*mut core::ffi::c_void) -> *mut core::ffi::c_void,
                               arg: *mut core::ffi::c_void) -> core::ffi::c_int;
  extern "C" fn pthread_exit(result: *mut core::ffi::c_void) -> !;
  extern "C" fn __pthread_create(res: *mut *mut core::ffi::c_void, attrp: *const core::ffi::c_void,
                                 entry: extern "C" fn(*mut core::ffi::c_void) -> *mut core::ffi::c_void,
                                 arg: *mut core::ffi::c_void) -> core::ffi::c_int;
  extern "C" fn __pthread_exit(result: *mut core::ffi::c_void) -> !;
  extern "C" fn __tl_lock();
  extern "C" fn __tl_unlock();
  extern "C" fn __tl_sync(td: *mut core::ffi::c_void);
  extern "C" fn __do_cleanup_push(cb: *mut Ptcb);
  extern "C" fn __do_cleanup_pop(cb: *mut Ptcb);
                                 // 本模块保证对外提供与 C ABI 兼容的上述符号

Internal Interface:
  pub(crate) unsafe fn thread_start(args: *mut StartArgs) -> !;
  pub(crate) unsafe fn thread_start_c11(args: *mut StartArgs) -> !;
  pub(crate) struct TlLockGuard;                                 // 线程列表锁 RAII 守卫
  pub(crate) struct CleanupStack;                                // 清理处理栈管理
  pub(crate) struct StartArgs;                                   // 线程启动参数（repr(C)）
  pub(crate) fn init_file_locks_on_first_thread();               // 首次线程化文件锁初始化
