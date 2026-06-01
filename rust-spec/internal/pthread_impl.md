# pthread_impl.rs 规约 (Rust)

> **来源 C spec**: `musl/src/internal/spec/pthread_impl.md`
> **对应源文件**: `musl/src/internal/pthread_impl.h`
> **复杂度层级**: Level 3 — 高度优化设计（线程控制块 ABI + TLS 管理 + futex 同步原语 + 信号系统交互）

---

## 依赖图

```
pthread (POSIX 标准类型)
  └── libc (rusl 内部: 全局 libc 状态)
        ├── syscall (rusl 内部: 系统调用封装)
        │     └── atomic (Rust core: AtomicI32, AtomicBool 等)
        ├── futex (rusl 内部: FUTEX_WAIT, FUTEX_WAKE, FUTEX_PRIVATE 等常量)
        ├── pthread_arch (架构相关: __get_tp(), TLS 布局常量)
        │     └── Pthread — 线程控制块 (TCB)
        ├── tls (TLS 函数: __tls_get_addr, __init_tp, __copy_tls, __reset_tls)
        ├── cancel (取消: __testcancel, __do_cleanup_push/pop)
        ├── sync (同步: __timedwait, __wait, __wake, __futexwait)
        ├── lock (锁: __acquire_ptc, __release_ptc, __inhibit_ptc, SpinLock)
        └── globals (全局变量: __thread_list_lock, __abort_lock, __pthread_tsd_size, __eintr_valid_flag)
```

---

## 概述

`pthread_impl` 模块是 rusl 线程子系统（POSIX pthread）的核心内部模块。它定义了：

1. **线程控制块 (TCB)** —— `Pthread` 结构体，是线程在内存中的完整表示，其布局受 ABI 约束
2. **线程同步原语** —— 基于 Linux futex 的等待/唤醒操作（`timedwait`、`wake`、`futexwait`）
3. **TLS（线程局部存储）** 管理 —— `tls_get_addr`、`init_tp`、`copy_tls`
4. **线程取消点** (cancellation point) 机制 —— `testcancel`、取消点版系统调用
5. **信号系统交互** —— `EINTR_VALID_FLAG`、内部信号常量 `SIGCANCEL`、`SIGTIMER`、`SIGSYNCCALL`
6. **架构抽象** —— 通过 `pthread_arch` 和 `syscall_arch` 模块隔离平台差异

**不变量 (Invariants)**：
- **I1**: `Pthread` 结构体的 Part 1 和 Part 3 字段偏移量是外部 ABI（编译器/运行时可见），**不得修改**。Part 2 字段为仅内部使用的实现细节。
- **I2**: 线程的 `self_` 指针必须指向自身的 `Pthread` 起始地址，`pthread_self()` 通过架构相关的 TP 寄存器检索此指针。
- **I3**: 线程链表（`prev`/`next`）构成双向全局链表，由 `THREAD_LIST_LOCK` 保护。
- **I4**: `detach_state` 只能按 `DT_EXITED -> DT_EXITING -> DT_JOINABLE` 或 `DT_DETACHED` 进行状态迁移。
- **I5**: 任何可阻塞的系统调用在调用前必须通过取消点检查（若线程启用了取消）。

---

## 核心数据结构

### `Pthread` — 线程控制块 (TCB)

```rust
// Rust 签名
#[repr(C)]
pub(crate) struct Pthread {
    /* Part 1 -- 这些字段的偏移量是外部 ABI，不得修改 */
    pub(crate) self_: *mut Pthread,            // 自引用指针
    #[cfg(not(TLS_ABOVE_TP))]
    pub(crate) dtv: *mut usize,                // Dynamic Thread Vector
    pub(crate) prev: *mut Pthread,             // 全局线程链表前驱
    pub(crate) next: *mut Pthread,             // 全局线程链表后继
    pub(crate) sysinfo: usize,                 // 系统信息（vsyscall 页地址等）
    #[cfg(not(TLS_ABOVE_TP))]
    #[cfg(CANARY_PAD)]
    pub(crate) canary_pad: usize,
    #[cfg(not(TLS_ABOVE_TP))]
    pub(crate) canary: usize,                  // 栈保护 canary

    /* Part 2 -- 实现细节，非 ABI，可自由调整布局 */
    pub(crate) tid: c_int,                     // 内核线程 ID
    pub(crate) errno_val: c_int,               // 线程局部的 errno
    pub(crate) detach_state: AtomicI32,         // 线程分离状态
    pub(crate) cancel: AtomicI32,               // 取消标志
    pub(crate) canceldisable: AtomicU8,         // 取消禁用计数
    pub(crate) cancelasync: AtomicU8,           // 异步取消启用标志
    pub(crate) tsd_used: bool,                  // 线程特定数据是否已初始化
    pub(crate) dlerror_flag: bool,             // dlopen/dlsym 错误标志
    pub(crate) map_base: *mut u8,              // 线程 mmap 区域起始
    pub(crate) map_size: usize,                // 线程 mmap 区域大小
    pub(crate) stack: *mut c_void,             // 线程栈基址
    pub(crate) stack_size: usize,              // 线程栈大小
    pub(crate) guard_size: usize,              // 保护页大小
    pub(crate) result: *mut c_void,            // 线程退出返回值
    pub(crate) cancelbuf: *mut Ptcb,           // 取消点清理处理链表头
    pub(crate) tsd: *mut *mut c_void,          // 线程特定数据数组
    pub(crate) robust_list: RobustList,         // robust mutex 链表
    pub(crate) h_errno_val: c_int,             // 线程局部的 h_errno
    pub(crate) timer_id: AtomicI32,             // 线程局部的定时器 ID
    pub(crate) locale: Locale,                  // 线程局部的 locale
    pub(crate) killlock: SpinLock,              // 信号递送锁
    pub(crate) dlerror_buf: *mut u8,           // 线程局部的 dlerror 缓冲区
    pub(crate) stdio_locks: *mut c_void,       // 线程持有的 stdio 锁链表头

    /* Part 3 -- 相对于结构体末尾的偏移量是外部 ABI */
    #[cfg(TLS_ABOVE_TP)]
    pub(crate) canary: usize,                  // 栈保护 canary
    #[cfg(TLS_ABOVE_TP)]
    pub(crate) dtv: *mut usize,                // Dynamic Thread Vector
}
```

[Visibility]: Internal — rusl 内部 TCB 定义。POSIX 标准中的 `pthread_t` 为 `*mut Pthread`（不透明指针）。

### `RobustList` — robust mutex 链表

```rust
// Rust 签名
#[repr(C)]
pub(crate) struct RobustList {
    pub(crate) head: *mut c_void,
    pub(crate) off: c_long,
    pub(crate) pending: *mut c_void,
}
```

[Visibility]: Internal

### `DetachState` — 线程分离状态枚举

```rust
// Rust 签名
#[repr(i32)]
pub(crate) enum DetachState {
    Exited = 0,    // 线程已退出（或尚未创建）
    Exiting = 1,   // 线程正在退出过程中
    Joinable = 2,  // 线程可被 pthread_join 等待
    Detached = 3,  // 线程已被分离，退出时自动释放资源
}
```

[Visibility]: Internal

**状态迁移图**：
```
Joinable ──[pthread_detach]──> Detached
Joinable ──[线程结束]──> Exiting ──> Exited
Detached  ──[线程结束]──> (自动释放资源，不保留 TCB)
```

---

## 线程同步原语

### Futex 操作

#### `wake(addr: *const AtomicI32, cnt: c_int, priv_: c_int)`

```rust
// Rust 签名
pub(crate) fn wake(addr: *const AtomicI32, cnt: c_int, priv_: c_int)
```

[Visibility]: Internal — rusl 内部 futex 唤醒操作

**意图**: 唤醒最多 `cnt` 个等待在地址 `addr` 上的 futex 等待者。

**前置条件**:
- `addr` 非空，指向一个有效的 `AtomicI32`（通常作为锁或条件变量）
- `cnt` 若为负数，则唤醒所有等待者（`INT_MAX`）

**后置条件**:
- 返回所述；唤醒的线程数不可直接获取
- 若 `FUTEX_WAKE | FUTEX_PRIVATE` 返回 `-ENOSYS`（旧内核），退化为 `FUTEX_WAKE`（不带 PRIVATE）

---

#### `futexwait(addr: *const AtomicI32, val: c_int, priv_: c_int)`

```rust
// Rust 签名
pub(crate) fn futexwait(addr: *const AtomicI32, val: c_int, priv_: c_int)
```

[Visibility]: Internal — rusl 内部 futex 等待操作

**意图**: 原子地比较 `*addr == val`，若相等则阻塞等待 futex 唤醒。若不等则立即返回。

**前置条件**:
- `addr` 非空，指向有效的 `AtomicI32`
- `val` 为期望的比较值

---

#### `timedwait(addr: *const AtomicI32, val: c_int, clk: clockid_t, at: *const timespec, priv_: c_int) -> c_int`

```rust
// Rust 签名
pub(crate) fn timedwait(
    addr: *const AtomicI32,
    val: c_int,
    clk: clockid_t,
    at: *const timespec,
    priv_: c_int,
) -> c_int
```

[Visibility]: Internal — rusl 内部带超时的 futex 等待

**意图**: 带超时的 futex 等待。若 `at` 为 NULL，则为无限等待。

**返回值**: 0 表示成功被唤醒，非零（如 `ETIMEDOUT`）表示超时。

---

#### `timedwait_cp(addr: *const AtomicI32, val: c_int, clk: clockid_t, at: *const timespec, priv_: c_int) -> c_int`

```rust
// Rust 签名
pub(crate) fn timedwait_cp(
    addr: *const AtomicI32,
    val: c_int,
    clk: clockid_t,
    at: *const timespec,
    priv_: c_int,
) -> c_int
```

[Visibility]: Internal — rusl 内部带取消点的超时 futex 等待

**意图**: 与 `timedwait` 相同，但内部调用 `testcancel()` 检查取消标志，是取消点（cancellation point）。

---

## TLS (线程局部存储) 管理

### 架构相关常量

```rust
// Rust 签名（编译期条件常量）
#[cfg(target_arch = "x86_64")]
pub(crate) const TP_OFFSET: isize = 0;
#[cfg(target_arch = "aarch64")]
pub(crate) const TP_OFFSET: isize = 0;

#[cfg(target_arch = "aarch64")]
pub(crate) const TLS_ABOVE_TP: bool = true;
#[cfg(target_arch = "x86_64")]
pub(crate) const TLS_ABOVE_TP: bool = false;
```

[Visibility]: Internal

**意图**: 不同 CPU 架构的线程指针约定不同。x86_64 中 TLS 在 TP 下方，aarch64 中 TLS 在 TP 上方。

---

### TLS 管理函数

| 函数 | Rust 签名 | 意图 |
|------|-----------|------|
| `tls_get_addr` | `pub(crate) fn tls_get_addr(v: *mut TlsModOff) -> *mut c_void` | TLS 变量地址的动态解析（延迟绑定 TLS 模型） |
| `init_tp` | `pub(crate) fn init_tp(tp: *mut c_void) -> c_int` | 初始化线程指针，设置主线程 TLS |
| `copy_tls` | `pub(crate) fn copy_tls(mem: *mut u8) -> *mut c_void` | 从 TLS 模板复制 TLS 数据到新线程的 TLS 区域 |
| `reset_tls` | `pub(crate) fn reset_tls()` | 子进程 fork 后重置 TLS 状态 |

[Visibility]: 全部 Internal

---

## 线程取消点机制

### 取消相关字段（位于 `Pthread` 结构体中）

```rust
pub(crate) cancel: AtomicI32,        // 取消请求标志
pub(crate) canceldisable: AtomicU8,  // 取消禁用计数（>0 时不能取消）
pub(crate) cancelasync: AtomicU8,    // 异步取消启用标志
```

### 取消管理函数

| 函数 | Rust 签名 | 意图 |
|------|-----------|------|
| `testcancel` | `pub(crate) fn testcancel()` | 检查取消标志，若已请求且未禁用，则执行 `pthread_exit(PTHREAD_CANCELED)` |
| `do_cleanup_push` | `pub(crate) fn do_cleanup_push(cb: *mut Ptcb)` | 将清理处理器压入取消清理栈 |
| `do_cleanup_pop` | `pub(crate) fn do_cleanup_pop(cb: *mut Ptcb)` | 从取消清理栈弹出并执行清理处理器 |

[Visibility]: 全部 Internal

---

## 线程创建/销毁辅助

### 默认常量

```rust
// Rust 签名
pub(crate) const DEFAULT_STACK_SIZE: usize = 131072;     // 默认栈大小 128KB
pub(crate) const DEFAULT_GUARD_SIZE: usize = 8192;        // 默认保护页大小 8KB
pub(crate) const DEFAULT_STACK_MAX: usize = 8 << 20;     // 最大栈大小 8MB
pub(crate) const DEFAULT_GUARD_MAX: usize = 1 << 20;     // 最大保护页大小 1MB
```

[Visibility]: Internal

### 可覆盖的默认值

```rust
// Rust 签名
pub(crate) static mut DEFAULT_STACKSIZE: c_uint = DEFAULT_STACK_SIZE as c_uint;
pub(crate) static mut DEFAULT_GUARDSIZE: c_uint = DEFAULT_GUARD_SIZE as c_uint;
```

[Visibility]: Internal — 可在 `pthread_attr_init` 时被 `ulimit` 或环境变量覆盖

---

### `ATTRP_C11_THREAD` 哨兵值

```rust
// Rust 签名
pub(crate) const ATTRP_C11_THREAD: *mut c_void = usize::MAX as *mut c_void;
```

[Visibility]: Internal

**意图**: 标记线程是通过 C11 `thrd_create()` 创建的（而非 POSIX `pthread_create()`），在 `thrd_join` 中需要特殊处理。

---

## 线程列表锁

### 锁操作

| 函数 | Rust 签名 | 意图 |
|------|-----------|------|
| `tl_lock` | `pub(crate) fn tl_lock()` | 获取线程列表锁 |
| `tl_unlock` | `pub(crate) fn tl_unlock()` | 释放线程列表锁 |
| `tl_sync` | `pub(crate) fn tl_sync(t: *mut Pthread)` | 同步等待目标线程的状态变更 |

[Visibility]: 全部 Internal

### 全局变量

```rust
// Rust 签名
pub(crate) static THREAD_LIST_LOCK: SpinLock = SpinLock::new();
```

[Visibility]: Internal — 保护全局线程链表（`prev`/`next`）的自旋锁

---

## PTC (PThread Create) 锁

| 函数 | Rust 签名 | 意图 |
|------|-----------|------|
| `acquire_ptc` | `pub(crate) fn acquire_ptc()` | 获取线程创建锁，阻止并发 `pthread_create` |
| `release_ptc` | `pub(crate) fn release_ptc()` | 释放线程创建锁 |
| `inhibit_ptc` | `pub(crate) fn inhibit_ptc()` | 临时禁止线程创建，用于 `fork()` 等关键区域 |

[Visibility]: 全部 Internal

---

## 信号系统交互

### 内部信号常量

```rust
// Rust 签名
pub(crate) const SIGTIMER: c_int = 32;     // musl 内部定时器信号
pub(crate) const SIGCANCEL: c_int = 33;    // musl 内部取消信号
pub(crate) const SIGSYNCCALL: c_int = 34;  // musl 内部同步调用信号
```

[Visibility]: Internal

**意图**: musl 使用实时信号来实现内部机制（不占用标准信号空间）：
- `SIGTIMER`: 线程级定时器到期通知
- `SIGCANCEL`: 向线程发送取消请求
- `SIGSYNCCALL`: `synccall` 的进程级同步屏障实现

---

### 信号集常量

```rust
// Rust 签名
pub(crate) const SIGALL_SET: Sigset = Sigset::all();       // 全信号集

// SIGPT_SET: 包含 SIGTIMER, SIGCANCEL, SIGSYNCCALL
pub(crate) const SIGPT_SET: Sigset = Sigset::from_bits_truncate(
    (1u64 << (SIGTIMER - 1)) | (1u64 << (SIGCANCEL - 1)) | (1u64 << (SIGSYNCCALL - 1))
);

// SIGTIMER_SET: 仅包含 SIGTIMER
pub(crate) const SIGTIMER_SET: Sigset = Sigset::from_bits_truncate(
    1u64 << (SIGTIMER - 1)
);
```

[Visibility]: Internal

---

## POSIX 同步对象字段访问

以下为 POSIX 同步对象（`pthread_mutex_t`、`pthread_cond_t`、`pthread_rwlock_t`、`pthread_barrier_t`）内部联合体的字段访问封装。

### Mutex 字段访问

```rust
// Rust 签名（对 PthreadMutex.__u 联合体的方法封装）
impl PthreadMutex {
    pub(crate) fn m_type(&self) -> c_int;
    pub(crate) fn m_lock(&self) -> &AtomicI32;
    pub(crate) fn m_waiters(&self) -> &AtomicI32;
    pub(crate) fn m_count(&self) -> c_int;
    // robust 链表 (仅 NORMAL 类型)
    pub(crate) fn m_prev(&self) -> *mut c_void;
    pub(crate) fn m_next(&self) -> *mut c_void;
}
```

### Cond 字段访问

```rust
// Rust 签名
impl PthreadCond {
    pub(crate) fn c_seq(&self) -> &AtomicI32;
    pub(crate) fn c_waiters(&self) -> &AtomicI32;
    pub(crate) fn c_clock(&self) -> c_int;
    pub(crate) fn c_lock(&self) -> &AtomicI32;
    pub(crate) fn c_head(&self) -> *mut c_void;
    pub(crate) fn c_tail(&self) -> *mut c_void;
}
```

### Rwlock 字段访问

```rust
// Rust 签名
impl PthreadRwlock {
    pub(crate) fn rw_lock(&self) -> &AtomicI32;
    pub(crate) fn rw_waiters(&self) -> &AtomicI32;
    pub(crate) fn rw_shared(&self) -> c_int;
}
```

### Barrier 字段访问

```rust
// Rust 签名
impl PthreadBarrier {
    pub(crate) fn b_lock(&self) -> &AtomicI32;
    pub(crate) fn b_waiters(&self) -> &AtomicI32;
    pub(crate) fn b_limit(&self) -> c_int;
    pub(crate) fn b_count(&self) -> &AtomicI32;
    pub(crate) fn b_waiters2(&self) -> &AtomicI32;
}
```

### 线程属性字段访问

```rust
// Rust 签名
impl PthreadAttr {
    pub(crate) fn a_stacksize(&self) -> usize;
    pub(crate) fn a_guardsize(&self) -> usize;
    pub(crate) fn a_stackaddr(&self) -> *mut c_void;
    pub(crate) fn a_detach(&self) -> c_int;
    pub(crate) fn a_sched(&self) -> c_int;
    pub(crate) fn a_policy(&self) -> c_int;
    pub(crate) fn a_prio(&self) -> c_int;
}
```

[Visibility]: 全部 Internal — 对 C 中宏访问的字段提供类型安全的 Rust 方法封装

---

## 杂项线程管理函数

| 函数 | Rust 签名 | 意图 |
|------|-----------|------|
| `membarrier_init` | `pub(crate) fn membarrier_init()` | 初始化内存屏障（若内核支持） |
| `dl_thread_cleanup` | `pub(crate) fn dl_thread_cleanup()` | 线程退出时的 dlopen 清理 |
| `pthread_tsd_run_dtors` | `pub(crate) fn pthread_tsd_run_dtors()` | 运行线程特定数据的析构函数 |
| `pthread_key_delete_synccall` | `pub(crate) fn pthread_key_delete_synccall(key: c_int)` | 通过 synccall 跨线程删除 TSD key |
| `clone` | `pub(crate) unsafe fn clone(...) -> c_int` | Linux `clone()` 系统调用的封装 |
| `set_thread_area` | `pub(crate) fn set_thread_area(p: *mut c_void) -> c_int` | 设置线程 TLS 区域（仅特定架构） |
| `libc_sigaction` | `pub(crate) fn libc_sigaction(sig: c_int, act: *const sigaction, oact: *mut sigaction) -> c_int` | libc 内部的 sigaction 包装 |
| `unmapself` | `pub(crate) unsafe fn unmapself(base: *mut c_void, size: usize) -> !` | 原子地 unmap 自身堆栈并退出 |

[Visibility]: 全部 Internal

---

### 全局变量

| 变量 | Rust 类型 | 意图 |
|------|-----------|------|
| `PTHREAD_TSD_SIZE` | `AtomicUsize` | 当前分配的 TSD 数组大小 |
| `PTHREAD_TSD_MAIN` | `[*mut c_void; TSD_MAIN_SIZE]` | 主线程的 TSD 数组（静态分配） |
| `EINTR_VALID_FLAG` | `AtomicI32` | 标记 `EINTR` 是否合法的全局标志 |
| `ABORT_LOCK` | `SpinLock` | `abort()` 信号安全的全局锁 |

[Visibility]: 全部 Internal

---

## 跨文件依赖

| 依赖符号 | 来源 | 处理方式 |
|---------|------|---------|
| `syscall` / `__syscall` | `syscall` 模块（rusl 内部） | 系统调用封装 |
| `AtomicI32`, `AtomicU8`, `AtomicUsize` | `core::sync::atomic` | Rust core 原子类型 |
| `FUTEX_WAIT`, `FUTEX_WAKE`, `FUTEX_PRIVATE` 等 | `futex` 模块（rusl 内部） | futex 操作码常量 |
| `__get_tp()` | `pthread_arch` 模块（架构相关） | 读取 TLS 寄存器 |
| `Libc` | `libc` 模块（rusl 内部） | 全局 libc 状态 |
| `SpinLock` | `lock` 模块（rusl 内部） | 自旋锁类型 |
| `Sigset` | `signal` 模块（rusl 内部） | 信号集类型 |
| `pthread_t` (`*mut Pthread`) | `pthread` 模块（rusl 内部） | POSIX 线程类型 |

---

## Rust 设计要点

- `Pthread` 使用 `#[repr(C)]` 确保 Part 1/3 ABI 字段偏移与 C 完全一致
- `detach_state` 使用 `AtomicI32` 替代 `volatile int`，提供精确的内存排序控制
- `DetachState` 使用 `#[repr(i32)]` 枚举，可与 `AtomicI32` 搭配使用 `compare_exchange`
- Futex 操作封装为安全函数，内部使用 `unsafe` 进行系统调用
- 信号集使用 `bitflags` 或自定义 bitset 类型，消除 C 复合字面量的可读性问题
- 同步对象的字段访问使用 `impl` 方法封装，替代 C 宏（`_m_lock` 等），提供类型安全和 IDE 支持
- `SpinLock` 支持 `const fn new()` 用于静态初始化全局锁
- TLS 管理通过 ELF 程序头解析实现，手动管理 TLS 模板复制和 `dtv` 表维护
- 架构差异通过 `#[cfg()]` 条件编译隔离，替代 C 的 `#ifdef`

---

## RELY / GUARANTEE

```
[RELY]
Rust Core 内建类型:
  core::sync::atomic::{AtomicI32, AtomicU8, AtomicUsize, Ordering}   // 原子操作
  core::ffi::{c_int, c_uint, c_long, c_void}                         // C FFI 类型

rusl 内部模块:
  syscall (系统调用封装)                                              // 依赖1: futex/线程 syscall
  futex (FUTEX 操作码常量)                                           // 依赖2: futex 常量
  pthread_arch (架构相关 TP 访问)                                     // 依赖3: __get_tp()
  lock::SpinLock (自旋锁)                                            // 依赖4: 内部锁原语
  signal::Sigset (信号集)                                            // 依赖5: 信号集类型
  libc (全局 libc 状态)                                              // 依赖6: Libc 结构体

[GUARANTEE]
pub(crate) 接口:
  struct Pthread                              // 线程控制块 (TCB)
  enum DetachState                            // 线程分离状态枚举
  struct RobustList                           // robust mutex 链表
  fn wake(addr, cnt, priv)                    // futex 唤醒
  fn futexwait(addr, val, priv)              // futex 等待
  fn timedwait(addr, val, clk, at, priv) -> c_int   // 带超时的 futex 等待
  fn timedwait_cp(addr, val, clk, at, priv) -> c_int // 带取消点的超时 futex 等待
  fn tls_get_addr(v) -> *mut c_void          // TLS 动态地址解析
  fn init_tp(tp) -> c_int                    // 初始化线程指针
  fn copy_tls(mem) -> *mut c_void             // 复制 TLS 模板
  fn reset_tls()                              // 复位 TLS
  fn testcancel()                             // 检查线程取消
  fn do_cleanup_push(cb)                      // 压入取消清理处理器
  fn do_cleanup_pop(cb)                       // 弹出取消清理处理器
  fn tl_lock() / tl_unlock() / tl_sync(t)     // 线程列表锁操作
  fn acquire_ptc() / release_ptc() / inhibit_ptc()  // PTC 锁操作
  fn membarrier_init()                        // 内存屏障初始化
  fn dl_thread_cleanup()                      // 线程退出 dlopen 清理
  fn pthread_tsd_run_dtors()                  // 运行 TSD 析构函数
  fn pthread_key_delete_synccall(key)         // 跨线程 TSD key 删除
  fn clone(...) -> c_int                      // clone 系统调用封装
  fn set_thread_area(p) -> c_int              // 设置 TLS 区域
  fn libc_sigaction(sig, act, oact) -> c_int  // libc sigaction
  fn unmapself(base, size) -> !               // unmap 并退出

  常量:
  DEFAULT_STACK_SIZE / DEFAULT_GUARD_SIZE     // 默认栈/保护页大小
  SIGTIMER / SIGCANCEL / SIGSYNCCALL          // 内部信号编号
  SIGALL_SET / SIGPT_SET / SIGTIMER_SET       // 信号集常量
  ATTRP_C11_THREAD                            // C11 线程哨兵

  同步对象访问器 (impl PthreadMutex / PthreadCond / PthreadRwlock / PthreadBarrier / PthreadAttr):
  各字段类型安全的 getter/setter 方法
```