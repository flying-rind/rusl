# pthread_impl.h 规约

> **来源文件**: `musl/src/internal/pthread_impl.h`
> **复杂度层级**: Level 3 — 高度优化设计（线程控制块 ABI + TLS 管理 + futex 同步原语 + 信号系统交互）
> **依赖图**:
> ```
> pthread.h, signal.h, errno.h, limits.h, sys/mman.h (标准 POSIX)
>   -> libc.h (musl 内部: __libc 结构体、线程状态)
>     -> syscall.h (musl 内部: __syscall, syscall 等宏)
>       -> atomic.h (musl 内部: a_cas, a_swap, a_inc, a_dec, a_store 等)
>         -> futex.h (musl 内部: FUTEX_WAIT, FUTEX_WAKE, FUTEX_PRIVATE 等常量)
>           -> pthread_arch.h (架构相关: __get_tp(), MC_PC, TLS 布局常量)
>             -> struct pthread — 线程控制块 (TCB)
>               -> 线程状态枚举 DT_EXITED/EXITING/JOINABLE/DETACHED
>                 -> TLS 函数: __tls_get_addr, __init_tp, __copy_tls, __reset_tls
>                   -> 同步/取消: __testcancel, __do_cleanup_push/pop, __timedwait, __wait, __wake, __futexwait
>                     -> 锁/信号: __acquire_ptc, __release_ptc, __inhibit_ptc, __tl_lock/unlock/sync
>                       -> 全局变量: __thread_list_lock, __abort_lock, __pthread_tsd_size, __eintr_valid_flag
> ```

---

## 概述

`pthread_impl.h` 是 musl libc 线程子系统（POSIX pthread）的核心内部头文件。它定义了：
1. **线程控制块 (TCB)** —— `struct pthread`，是线程在内存中的完整表示，其布局受 ABI 约束
2. **线程同步原语** —— 基于 Linux futex 的等待/唤醒操作（`__timedwait`、`__wake`、`__futexwait`）
3. **TLS（线程局部存储）** 管理 —— `__tls_get_addr`、`__init_tp`、`__copy_tls`
4. **线程取消点** (cancellation point) 机制 —— `__testcancel`、取消点版系统调用
5. **信号系统交互** —— `__eintr_valid_flag`、信号集常量、`SIGCANCEL`、`SIGTIMER`、`SIGSYNCCALL`
6. **架构抽象** —— 通过 `pthread_arch.h` 和 `syscall_arch.h` 隔离平台差异

**不变量 (Invariants)**：
- **I1**: `struct pthread` 的 Part 1 和 Part 3 字段偏移量是外部 ABI（编译器/运行时可见），**不得修改**。Part 2 字段仅内部使用的实现细节。
- **I2**: 线程的 `self` 指针必须指向自身的 `struct pthread` 起始地址，`__pthread_self()` 通过 `__get_tp()` 检索此指针。
- **I3**: 线程链表 (`prev`/`next`) 构成双向全局链表，由 `__thread_list_lock` 保护。
- **I4**: `detach_state` 只能按 `DT_EXITED → DT_EXITING → DT_JOINABLE` 或 `DT_DETACHED` 进行状态迁移。
- **I5**: 任何可阻塞的系统调用在调用前必须通过取消点检查（若线程启用了取消）。

---

## 核心数据结构

### `struct pthread`

```c
struct pthread {
    /* Part 1 -- 这些字段可能是外部或内部 ABI，不得修改 */
    struct pthread *self;
#ifndef TLS_ABOVE_TP
    uintptr_t *dtv;
#endif
    struct pthread *prev, *next;
    uintptr_t sysinfo;
#ifndef TLS_ABOVE_TP
#ifdef CANARY_PAD
    uintptr_t canary_pad;
#endif
    uintptr_t canary;
#endif

    /* Part 2 -- 实现细节，非 ABI */
    int tid;
    int errno_val;
    volatile int detach_state;
    volatile int cancel;
    volatile unsigned char canceldisable, cancelasync;
    unsigned char tsd_used:1;
    unsigned char dlerror_flag:1;
    unsigned char *map_base;
    size_t map_size;
    void *stack;
    size_t stack_size;
    size_t guard_size;
    void *result;
    struct __ptcb *cancelbuf;
    void **tsd;
    struct {
        volatile void *volatile head;
        long off;
        volatile void *volatile pending;
    } robust_list;
    int h_errno_val;
    volatile int timer_id;
    locale_t locale;
    volatile int killlock[1];
    char *dlerror_buf;
    void *stdio_locks;

    /* Part 3 -- 相对于结构体末尾的偏移量是外部和内部 ABI */
#ifdef TLS_ABOVE_TP
    uintptr_t canary;
    uintptr_t *dtv;
#endif
};
```

[Visibility]: Internal — musl 内部 TCB 定义；POSIX 标准中的 `pthread_t` 为不透明类型，实际是 `struct __pthread *`

**字段详细语义**：

| 分组 | 字段 | 类型 | 语义 |
|------|------|------|------|
| **Part 1 (ABI)** | `self` | `struct pthread *` | 自引用指针，必须指向本结构体起始地址 |
| | `dtv` (TP 下方架构) | `uintptr_t *` | Dynamic Thread Vector — TLS 模块表的指针 |
| | `prev`, `next` | `struct pthread *` | 全局线程链表的前驱/后继 |
| | `sysinfo` | `uintptr_t` | 系统信息（如 vsyscall 页地址等） |
| | `canary` | `uintptr_t` | 栈保护 canary 值（栈溢出检测） |
| **Part 2 (非 ABI)** | `tid` | `int` | 内核线程 ID（`getpid()` 返回值） |
| | `errno_val` | `int` | 线程局部的 errno 值 |
| | `detach_state` | `volatile int` | 线程分离状态（`DT_*` 枚举） |
| | `cancel` | `volatile int` | 取消标志（非零表示已被请求取消） |
| | `canceldisable` | `volatile unsigned char` | 取消禁用计数器 |
| | `cancelasync` | `volatile unsigned char` | 异步取消启用标志 |
| | `tsd_used` | `unsigned char:1` | 线程特定数据是否已初始化 |
| | `dlerror_flag` | `unsigned char:1` | dlopen/dlsym 错误标志 |
| | `map_base` | `unsigned char *` | 线程 mmap 区域的起始（用于 TCB 内存释放） |
| | `map_size` | `size_t` | 线程 mmap 区域的大小 |
| | `stack` | `void *` | 线程栈基址指针 |
| | `stack_size` | `size_t` | 线程栈大小 |
| | `guard_size` | `size_t` | 线程栈保护页大小 |
| | `result` | `void *` | 线程退出返回值（供 `pthread_join` 获取） |
| | `cancelbuf` | `struct __ptcb *` | 取消点清理处理链表的头 |
| | `tsd` | `void **` | 线程特定数据数组指针 |
| | `robust_list` | 匿名结构体 | robust mutex 链表（用于进程死亡后的 mutex 回收） |
| | `h_errno_val` | `int` | 线程局部的 h_errno 值（DNS 解析错误码） |
| | `timer_id` | `volatile int` | 线程局部的定时器 ID |
| | `locale` | `locale_t` | 线程局部的 locale 设置 |
| | `killlock` | `volatile int[1]` | 信号递送锁（防止信号处理中的重入） |
| | `dlerror_buf` | `char *` | 线程局部的 dlerror 缓冲区 |
| | `stdio_locks` | `void *` | 线程持有的 stdio 锁链表头 |
| **Part 3 (ABI)** | `canary` (TP 上方架构) | `uintptr_t` | 栈保护 canary（位置因 TLS 布局而异） |
| | `dtv` (TP 上方架构) | `uintptr_t *` | Dynamic Thread Vector |

---

## 线程状态枚举

```c
enum {
    DT_EXITED = 0,    // 线程已退出（或尚未创建）
    DT_EXITING,       // 线程正在退出过程中
    DT_JOINABLE,      // 线程可被 pthread_join 等待
    DT_DETACHED,      // 线程已被分离，退出时自动释放资源
};
```

**状态迁移图**：
```
DT_JOINABLE ──[pthread_detach]──> DT_DETACHED
DT_JOINABLE ──[线程结束]──> DT_EXITING ──> DT_EXITED
DT_DETACHED  ──[线程结束]──> (自动释放资源，不保留 TCB)
```

---

## TLS (线程局部存储) 相关

### 架构相关宏

```c
#define TP_OFFSET 0          // Thread Pointer 与 TCB 末尾的偏移
#define DTP_OFFSET 0         // Dynamic Thread Pointer 的偏移
#define TLS_ABOVE_TP         // 若定义，TLS 数据在 TP 地址上方（否则在下方）
```

**意图**: 不同 CPU 架构的线程指针约定不同。例如：
- x86_64: TLS 在 TP 下方，`self` 在 TCB 头部，canary 在 TCB 中间
- aarch64: TLS 在 TP 上方，canary 和 dtv 在 TCB 末尾

### `TP_ADJ(p)` / `__pthread_self()`

```c
#ifdef TLS_ABOVE_TP
#define TP_ADJ(p) ((char *)(p) + sizeof(struct pthread) + TP_OFFSET)
#define __pthread_self() ((pthread_t)(__get_tp() - sizeof(struct __pthread) - TP_OFFSET))
#else
#define TP_ADJ(p) (p)
#define __pthread_self() ((pthread_t)__get_tp())
#endif
```

[Visibility]: Internal — musl 内部宏/函数

**意图**:
- `TP_ADJ(p)`: 从 TCB 指针计算线程指针（Thread Pointer）值，用于设置 TLS 寄存器
- `__pthread_self()`: 从 TLS 寄存器值反向计算 `struct pthread *`，返回当前线程的控制块指针

### TLS 管理函数

| 函数 | 签名 | 意图 |
|------|------|------|
| `__tls_get_addr` | `void *__tls_get_addr(tls_mod_off_t *)` | TLS 变量地址的动态解析（延迟绑定 TLS 模型） |
| `__init_tp` | `int __init_tp(void *)` | 初始化线程指针，设置主线程 TLS |
| `__copy_tls` | `void *__copy_tls(unsigned char *)` | 从 TLS 模板复制 TLS 数据到新线程的 TLS 区域 |
| `__reset_tls` | `void __reset_tls()` | 子进程 fork 后重置 TLS 状态 |

---

## 线程同步原语

### Futex 操作（内联函数）

#### `__wake(volatile void *, int, int)`

```c
static inline void __wake(volatile void *addr, int cnt, int priv)
{
    if (priv) priv = FUTEX_PRIVATE;
    if (cnt<0) cnt = INT_MAX;
    __syscall(SYS_futex, addr, FUTEX_WAKE|priv, cnt) != -ENOSYS ||
    __syscall(SYS_futex, addr, FUTEX_WAKE, cnt);
}
```

[Visibility]: Internal — musl 内部 static inline 函数

**意图**: 唤醒最多 `cnt` 个等待在地址 `addr` 上的 futex 等待者。

**前置条件**:
- `addr` 指向共享内存中的合法地址（通常为 `volatile int *`）
- 若 `priv` 非 0，使用 `FUTEX_PRIVATE` 标志进行进程内优化

**后置条件**:
- 返回所述，唤醒的线程数不可直接获取
- 若 `FUTEX_WAKE|FUTEX_PRIVATE` 返回 `-ENOSYS`（旧内核），退化为 `FUTEX_WAKE`（不带 PRIVATE）

---

#### `__futexwait(volatile void *, int, int)`

```c
static inline void __futexwait(volatile void *addr, int val, int priv)
{
    if (priv) priv = FUTEX_PRIVATE;
    __syscall(SYS_futex, addr, FUTEX_WAIT|priv, val, 0) != -ENOSYS ||
    __syscall(SYS_futex, addr, FUTEX_WAIT, val, 0);
}
```

[Visibility]: Internal — musl 内部 static inline 函数

**意图**: 原子地比较 `*addr == val`，若相等则阻塞等待 futex 唤醒。

**前置条件**:
- `addr` 指向有效的 `volatile int` 变量
- `val` 为期望的比较值（若 `*addr != val` 则立即返回不阻塞）

---

### Futex 包装函数

| 函数 | 签名 | 意图 |
|------|------|------|
| `__timedwait` | `int __timedwait(volatile int *, int, clockid_t, const struct timespec *, int)` | 带超时的 futex 等待，返回 0 成功/非零超时 |
| `__timedwait_cp` | `int __timedwait_cp(volatile int *, int, clockid_t, const struct timespec *, int)` | 带取消点的超时 futex 等待（cancel point） |
| `__wait` | `void __wait(volatile int *, volatile int *, int, int)` | 无条件 futex 等待（内部使用 `a_dec` + `__futexwait`） |

---

## 线程取消点机制

### 取消点标志字段

```c
volatile int cancel;              // 取消请求标志
volatile unsigned char canceldisable;  // 取消禁用计数（>0 时不能取消）
volatile unsigned char cancelasync;    // 异步取消启用标志
```

### 关键函数

| 函数 | 意图 |
|------|------|
| `__testcancel()` | 检查取消标志，若已请求且未禁用，则执行 `pthread_exit(PTHREAD_CANCELED)` |
| `__do_cleanup_push(struct __ptcb *)` | 将清理处理器压入取消清理栈 |
| `__do_cleanup_pop(struct __ptcb *)` | 从取消清理栈弹出并执行清理处理器 |

---

## 线程创建/销毁辅助

### 线程属性相关的宏

```c
#define DEFAULT_STACK_SIZE 131072     // 默认栈大小 128KB
#define DEFAULT_GUARD_SIZE 8192       // 默认保护页大小 8KB
#define DEFAULT_STACK_MAX (8<<20)     // 最大栈大小 8MB
#define DEFAULT_GUARD_MAX (1<<20)     // 最大保护页大小 1MB

extern hidden unsigned __default_stacksize;
extern hidden unsigned __default_guardsize;
```

**意图**: 默认值和运行时可配置的最大值分离。`__default_stacksize` 和 `__default_guardsize` 可在 `pthread_attr_init` 时被 `ulimit` 或环境变量覆盖。

### `__ATTRP_C11_THREAD`

```c
#define __ATTRP_C11_THREAD ((void*)(uintptr_t)-1)
```

**意图**: 哨兵值，标记线程是通过 C11 `thrd_create()` 创建的（而非 POSIX `pthread_create()`），在 `thrd_join` 中需要特殊处理。

---

## 线程列表锁

### 锁操作

| 函数 | 意图 |
|------|------|
| `__tl_lock()` | 获取线程列表锁，阻塞等待 |
| `__tl_unlock()` | 释放线程列表锁 |
| `__tl_sync(pthread_t)` | 同步等待目标线程的状态变更 |

### 全局变量

```c
extern hidden volatile int __thread_list_lock;
```

**意图**: 保护全局线程链表 (`prev`/`next`) 和 `__libc.threads_minus_1` 的自旋锁。

---

## 信号系统交互

### 内部信号定义

```c
#define SIGTIMER    32    // musl 内部定时器信号
#define SIGCANCEL   33    // musl 内部取消信号
#define SIGSYNCCALL 34    // musl 内部同步调用信号
```

**意图**: musl 使用实时信号来实现内部机制（不占用标准信号空间）：
- `SIGTIMER`: 线程级定时器到期通知
- `SIGCANCEL`: 向线程发送取消请求
- `SIGSYNCCALL`: `__synccall` 的进程级同步屏障实现

### 信号集宏

```c
#define SIGALL_SET ((sigset_t *)(const unsigned long long [2]){ -1,-1 })
#define SIGPT_SET  ((sigset_t *)(const unsigned long [_NSIG/8/sizeof(long)]){ \
    [sizeof(long)==4] = 3UL<<(32*(sizeof(long)>4)) })
#define SIGTIMER_SET ((sigset_t *)(const unsigned long [_NSIG/8/sizeof(long)]){ \
    0x80000000 })
```

**意图**: 在栈上构造复合字面量信号集：
- `SIGALL_SET`: 全信号集（所有位为 1）
- `SIGPT_SET`: 包含 `SIGTIMER`、`SIGCANCEL`、`SIGSYNCCALL` 这三个内部信号
- `SIGTIMER_SET`: 仅包含 `SIGTIMER`

---

## pthread_sigmask 结构体字段宏

以下宏定义了对 POSIX 同步对象（`pthread_mutex_t`、`pthread_cond_t`、`pthread_rwlock_t`、`pthread_barrier_t`）内部联合体 `__u` 的字段访问别名。

### mutex 字段

```c
#define _m_type   __u.__i[0]    // mutex 类型 (NORMAL/RECURSIVE/ERRORCHECK)
#define _m_lock   __u.__vi[1]   // mutex 锁变量 (futex)
#define _m_waiters __u.__vi[2]  // 等待者计数
#define _m_prev   __u.__p[3]    // robust 链表前驱
#define _m_next   __u.__p[4]    // robust 链表后继
#define _m_count  __u.__i[5]    // 递归计数
```

### cond 字段

```c
#define _c_shared  __u.__p[0]   // 共享 mutex 指针
#define _c_seq     __u.__vi[2]  // 条件变量序列号
#define _c_waiters __u.__vi[3]  // 等待者计数
#define _c_clock   __u.__i[4]   // 时钟 ID (CLOCK_REALTIME/CLOCK_MONOTONIC)
#define _c_lock    __u.__vi[8]  // 内部锁
#define _c_head    __u.__p[1]   // 等待队列头
#define _c_tail    __u.__p[5]   // 等待队列尾
```

### rwlock 字段

```c
#define _rw_lock    __u.__vi[0]  // 读写锁变量
#define _rw_waiters __u.__vi[1]  // 等待者计数
#define _rw_shared  __u.__i[2]   // 共享模式标志
```

### barrier 字段

```c
#define _b_lock     __u.__vi[0]  // barrier 锁
#define _b_waiters  __u.__vi[1]  // 未到达线程计数
#define _b_limit    __u.__i[2]   // barrier 目标计数值
#define _b_count    __u.__vi[3]  // 当前到达计数
#define _b_waiters2 __u.__vi[4]  // 第二波等待者计数
#define _b_inst     __u.__p[3]   // barrier 实例指针
```

### 线程属性字段

```c
#define __SU (sizeof(size_t)/sizeof(int))    // size_t/int 的比值（32位=1，64位=2）
#define _a_stacksize  __u.__s[0]    // 栈大小
#define _a_guardsize  __u.__s[1]    // 保护页大小
#define _a_stackaddr  __u.__s[2]    // 栈地址
#define _a_detach     __u.__i[3*__SU+0]    // 分离状态
#define _a_sched      __u.__i[3*__SU+1]    // 调度策略
#define _a_policy     __u.__i[3*__SU+2]    // 调度参数
#define _a_prio       __u.__i[3*__SU+3]    // 优先级
```

**意图**: 由于 `pthread_mutex_t` 等 POSIX 类型在 musl 中定义为含有 `__u` 匿名联合体的结构体，这些宏为源码提供人类可读的字段别名。`__vi` 表示 `volatile int` 数组，`__i` 表示 `int` 数组，`__p` 表示 `void *` 数组，`__s` 表示 `size_t` 数组。

---

## PTC (PThread Create) 锁

| 函数 | 意图 |
|------|------|
| `__acquire_ptc()` | 获取线程创建锁，阻止并发 `pthread_create` |
| `__release_ptc()` | 释放线程创建锁 |
| `__inhibit_ptc()` | 临时禁止线程创建，用于 `fork()` 等关键区域 |

---

## 杂项线程管理函数

| 函数 | 意图 |
|------|------|
| `__membarrier_init()` | 初始化内存屏障（若内核支持） |
| `__dl_thread_cleanup()` | 线程退出时的 dlopen 清理 |
| `__pthread_tsd_run_dtors()` | 运行线程特定数据的析构函数 |
| `__pthread_key_delete_synccall()` | 通过 synccall 跨线程删除 TSD key |
| `__pthread_key_delete_impl()` | TSD key 删除的底层实现 |
| `__clone()` | 类似 Linux `clone()` 系统调用的封装 |
| `__set_thread_area()` | 设置线程 TLS 区域（仅特定架构） |
| `__libc_sigaction()` | libc 内部的 sigaction 包装（使用实时信号） |
| `__unmapself()` | 原子地 unmap 自身堆栈并退出（线程退出时的最终步骤） |

### 全局变量

| 变量 | 类型 | 意图 |
|------|------|------|
| `__pthread_tsd_size` | `volatile size_t` | 当前分配的 TSD 数组大小 |
| `__pthread_tsd_main` | `void *[]` | 主线程的 TSD 数组 |
| `__eintr_valid_flag` | `volatile int` | 标记 `EINTR` 是否合法的全局标志 |
| `__abort_lock` | `volatile int[1]` | `abort()` 信号安全的全局锁 |
| `__default_stacksize` | `unsigned` | 默认线程栈大小（可被 ulimit 覆盖） |
| `__default_guardsize` | `unsigned` | 默认保护页大小 |

---

## 跨文件依赖

| 依赖符号 | 来源 | 处理方式 |
|---------|------|---------|
| `__syscall` / `syscall` | `syscall.h`（musl 内部） | 系统调用宏（见 syscall.h spec） |
| `a_cas`, `a_swap`, `a_inc`, `a_dec`, `a_store`, `a_spin` | `atomic.h`（musl 内部） | 原子操作（见 atomic.h spec） |
| `FUTEX_WAIT`, `FUTEX_WAKE`, `FUTEX_PRIVATE` 等 | `futex.h`（musl 内部） | futex 操作码常量 |
| `__get_tp()` | `pthread_arch.h`（架构相关） | 读取 TLS 寄存器（如 x86_64 的 `mov %%fs:0`） |
| `struct __libc` | `libc.h`（musl 内部） | 全局 libc 状态 |
| `struct tls_module` | `libc.h`（musl 内部） | TLS 模块描述 |
| `__synccall()` | `libc.h`（musl 内部） | 进程级同步屏障 |
| `struct __locale_struct` | `libc.h`（musl 内部） | locale 结构（通过 `locale_t` 引用） |
| `<pthread.h>` | POSIX 标准头文件 | `pthread_t`、`pthread_attr_t` 等类型定义 |

---

## 实现指南 (rusl/Rust)

- `struct pthread` → `#[repr(C)]` Rust 结构体。Part 1/3 字段偏移必须与 C ABI 完全一致。
- `__pthread_self()` → 使用 Rust 的线程局部存储或内联汇编获取 TP 寄存器值。
- `__wake` / `__futexwait` → 调用 Linux `futex` 系统调用的内联函数，带私有 futex 降级逻辑。
- `detach_state` → Rust `enum DetachState { Exited, Exiting, Joinable, Detached }` + `AtomicI32`。
- 线程链表 → `Mutex<LinkedList>` 或在 Rust 中直接用 `prev`/`next` 裸指针维护 + 自旋锁保护。
- 取消点 → 在阻塞 syscall 前使用 `check_cancel()` 检查；`__testcancel()` 对应 `pthread_exit()`。
- 信号集常量 → Rust `const` 或 `lazy_static` / `once_cell`。
- `__clone` → 使用 Linux `clone` 或 `clone3` 系统调用。注意 Rust 中 fork/clone 相关的安全性问题，需要在 unsafe 块中处理堆栈和 TLS 初始化。
- TLS 管理 → 解析 ELF 的 TLS 程序头，手动管理 TLS 模板复制和 `dtv` 表维护。
- `syscall_arch.h` → 使用 Rust `asm!` 宏或 `sc` crate（若可用）实现架构级 syscall 内联汇编。