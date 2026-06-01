# fork_impl.h 规约

## 概述

`fork_impl.h` 声明了 musl 在多线程环境下 `fork()` 的安全实现所需的所有锁指针和 atfork 回调函数。由于 `fork()` 通过系统调用复制整个进程地址空间，但在多线程程序中，其他线程可能持有锁，子进程中的这些锁将永远无法释放。musl 的解决方案是：在 fork 前获取所有全局锁，在 fork 后在父子进程中分别释放或重置锁。

## 依赖图

```
fork_impl.h
├── <features.h> (标准头)
│
├── [锁指针] (声明于本文件，定义于各 .c 模块)
│   ├── __at_quick_exit_lockptr    → 定义于 exit/at_quick_exit.c
│   ├── __atexit_lockptr           → 定义于 exit/atexit.c
│   ├── __gettext_lockptr          → 定义于 locale/gettext.c (若存在)
│   ├── __locale_lockptr           → 定义于 locale/locale_map.c
│   ├── __random_lockptr           → 定义于 prng/random.c
│   ├── __sem_open_lockptr         → 定义于 sem_open.c
│   ├── __stdio_ofl_lockptr        → 定义于 stdio/__ofl.c
│   ├── __syslog_lockptr           → 定义于 misc/syslog.c
│   ├── __timezone_lockptr         → 定义于 time/__tz.c
│   ├── __bump_lockptr             → 定义于 malloc/malloc.c (bump allocator)
│   └── __vmlock_lockptr           → 定义于 mmap 相关
│
├── [Atfork 回调] (声明于本文件，定义于各模块)
│   ├── __malloc_atfork(int)       → 定义于 src/malloc/malloc.c
│   ├── __ldso_atfork(int)         → 定义于 ldso/dynlink.c
│   └── __pthread_key_atfork(int)  → 定义于 thread/pthread_key_create.c
│
└── [Fork 后处理]
    └── __post_Fork(int)           → 定义于 process/fork.c
```

---

## 类型规约

### 锁指针类型
```c
volatile int *const
```
```
// Rust 等价: 指向 AtomicI32 的不可变指针
// 实际类型为 &AtomicI32, 不可重新绑定但指向的值可变
```

[Visibility]: Internal — musl 内部锁机制，非 POSIX 标准定义

**Intent**: 每个锁指针指向一个 `int` 类型的 futex 字，用作该模块的全局锁。`volatile` 防止编译器优化掉对锁的访问，`const` 确保指针本身不可被重新赋值。指针指向的 `int` 值可变，在持有锁时为持有线程的 TID，锁空闲时为 0。

---

## 符号规约

### 锁指针组 (11 个全局锁指针)

#### __at_quick_exit_lockptr
```c
extern hidden volatile int *const __at_quick_exit_lockptr;
```
```
// Rust: pub static __at_quick_exit_lockptr: &AtomicI32;
```

[Visibility]: Internal — musl 内部锁，保护 `at_quick_exit` 处理函数链表的并发访问

**Pre/Post 条件**: 不适用于变量声明本身。其指向的锁遵循标准 futex 锁协议：0 表示空闲，非零表示由该 TID 持有。

**Intent**: 保护 `quick_exit` / `at_quick_exit` 函数注册链表的并发修改。

---

#### __atexit_lockptr
```c
extern hidden volatile int *const __atexit_lockptr;
```
```
// Rust: pub static __atexit_lockptr: &AtomicI32;
```

[Visibility]: Internal — musl 内部锁，保护 `atexit` 处理函数链表

**Intent**: 保护 `exit` / `atexit` 函数注册链表的并发修改。

---

#### __gettext_lockptr
```c
extern hidden volatile int *const __gettext_lockptr;
```
```
// Rust: pub static __gettext_lockptr: &AtomicI32;
```

[Visibility]: Internal — musl 内部锁，保护 gettext 国际化数据

**Intent**: 保护 gettext / 国际化消息目录的并发加载与访问。

---

#### __locale_lockptr
```c
extern hidden volatile int *const __locale_lockptr;
```
```
// Rust: pub static __locale_lockptr: &AtomicI32;
```

[Visibility]: Internal — musl 内部锁，保护 locale 数据结构

**Intent**: 保护 `struct __locale_struct` 的并发修改（`setlocale` 调用等）。

---

#### __random_lockptr
```c
extern hidden volatile int *const __random_lockptr;
```
```
// Rust: pub static __random_lockptr: &AtomicI32;
```

[Visibility]: Internal — musl 内部锁，保护随机数生成器状态

**Intent**: 保护 `random()` / `srandom()` 内部 PRNG 状态的并发访问。

---

#### __sem_open_lockptr
```c
extern hidden volatile int *const __sem_open_lockptr;
```
```
// Rust: pub static __sem_open_lockptr: &AtomicI32;
```

[Visibility]: Internal — musl 内部锁，保护命名信号量全局列表

**Intent**: 保护 `sem_open` / `sem_close` / `sem_unlink` 中命名信号量全局注册表。

---

#### __stdio_ofl_lockptr
```c
extern hidden volatile int *const __stdio_ofl_lockptr;
```
```
// Rust: pub static __stdio_ofl_lockptr: &AtomicI32;
```

[Visibility]: Internal — musl 内部锁，保护 stdio 打开文件列表

**Intent**: 保护 `FILE` 结构体打开的全局文件链表（`OFLLock`），在 `fopen` / `fclose` 中用于添加/移除文件。

---

#### __syslog_lockptr
```c
extern hidden volatile int *const __syslog_lockptr;
```
```
// Rust: pub static __syslog_lockptr: &AtomicI32;
```

[Visibility]: Internal — musl 内部锁，保护 syslog 连接

**Intent**: 保护 `syslog()` 内部使用的 Unix 域套接字连接状态。

---

#### __timezone_lockptr
```c
extern hidden volatile int *const __timezone_lockptr;
```
```
// Rust: pub static __timezone_lockptr: &AtomicI32;
```

[Visibility]: Internal — musl 内部锁，保护时区数据

**Intent**: 保护时区相关全局变量的并发访问。

---

#### __bump_lockptr
```c
extern hidden volatile int *const __bump_lockptr;
```
```
// Rust: pub static __bump_lockptr: &AtomicI32;
```

[Visibility]: Internal — musl 内部锁，保护 bump allocator

**Intent**: 保护 musl 内部使用的线性 bump 分配器，用于分配库内部的小块内存。参见 `tre-mem.c` 的 `tre_mem_new` 实现。

---

#### __vmlock_lockptr
```c
extern hidden volatile int *const __vmlock_lockptr;
```
```
// Rust: pub static __vmlock_lockptr: &AtomicI32;
```

[Visibility]: Internal — musl 内部锁，保护虚拟内存操作

**Intent**: 保护 `mmap` / `munmap` 相关内部状态，防止 fork 时内存映射状态不一致。

---

### Atfork 回调函数

#### __malloc_atfork
```c
hidden void __malloc_atfork(int who);
```
```rust
// Rust
fn __malloc_atfork(who: c_int);
```

[Visibility]: Internal — musl 内部 atfork 回调，由 `fork()` 实现调用

**前置条件**:
- `who` 必须属于 {`-1`, `0`, `1`}，分别表示：fork 前准备(prepare)、父进程恢复(parent)、子进程恢复(child)
- 该函数在 `fork()` 系统调用的关键区段被调用，调用者持有进程内所有锁或即将获取它们

**后置条件**:
- `who == -1` (prepare): 获取 malloc 全局锁，确保 fork 期间 malloc 内部状态一致
- `who == 0` (parent): 释放 malloc 全局锁
- `who == 1` (child): 重置 malloc 全局锁为空闲状态（子进程中无其他线程持有锁）

**Intent**: 确保 `fork()` 后子进程的 malloc 实现处于一致状态。由于子进程继承了父进程的全部内存，但只有调用 `fork()` 的线程存在，其他线程持有的 malloc 内部锁在子进程中变成死锁。此回调在 fork 前后协调锁状态。

**System Algorithm**: 
1. prepare 阶段 (`who == -1`): 获取所有自定义 arena 的锁
2. parent 阶段 (`who == 0`): 释放 prepare 阶段获取的所有锁
3. child 阶段 (`who == 1`): 重置所有锁为空闲，清理 arena 的线程关联信息

---

#### __ldso_atfork
```c
hidden void __ldso_atfork(int who);
```
```rust
fn __ldso_atfork(who: c_int);
```

[Visibility]: Internal — musl 内部 atfork 回调，由动态链接器提供

**前置条件**:
- `who` ∈ {`-1`, `0`, `1`}
- 调用者必须处于 `fork()` 的关键区段

**后置条件**:
- `who == -1`: 获取动态链接器内部锁
- `who == 0`: 释放动态链接器锁
- `who == 1`: 重置动态链接器锁（子进程中无其他线程）

**Intent**: 保护动态链接器的内部数据结构（如加载的共享库链表）在 fork 期间的一致性。

---

#### __pthread_key_atfork
```c
hidden void __pthread_key_atfork(int who);
```
```rust
fn __pthread_key_atfork(who: c_int);
```

[Visibility]: Internal — musl 内部 atfork 回调，由 pthread 实现提供

**前置条件**:
- `who` ∈ {`-1`, `0`, `1`}
- 仅在多线程程序中有效（`libc.threads_minus_1 >= 0`）

**后置条件**:
- `who == -1`: 获取 pthread key 内部锁
- `who == 0`: 释放 pthread key 锁
- `who == 1`: 重置 pthread key 锁，重置 TSD (Thread-Specific Data) 析构链表

**Intent**: 保护 pthread 线程特定数据 (TSD / TLS key) 在 fork 后的正确性。

---

### Fork 后处理

#### __post_Fork
```c
hidden void __post_Fork(int ret);
```
```rust
fn __post_Fork(ret: c_int);
```

[Visibility]: Internal — musl 内部，`fork()` 系统调用返回后立即调用

**前置条件**:
- `ret` 为 `fork()` 系统调用的返回值
- `ret == 0` 表示当前在子进程
- `ret > 0` 表示当前在父进程，`ret` 值为子进程 PID
- `ret < 0` 表示 fork 失败

**后置条件**:
- **Case 1 — 父进程 (`ret > 0`)**: 
  - 释放 prepare 阶段获取的所有全局锁（恢复到 fork 前状态）
  - 父进程的线程调度和同步恢复正常
- **Case 2 — 子进程 (`ret == 0`)**:
  - 重置所有全局锁为空闲状态（因为子进程仅有一个线程）
  - 重置 libc 线程计数器 `__libc.threads_minus_1 = 0`
  - 调用所有 atfork 子进程回调
  - 清理其他继承自父进程的状态（如 robust mutex 列表）
- **Case 3 — fork 失败 (`ret < 0`)**:
  - 释放 prepare 阶段获取的所有锁
  - 设置 `errno` 并返回

**System Algorithm**: 该函数按固定顺序处理：
1. 按持有锁的逆序（`__bump_lockptr` → `__vmlock_lockptr` → `__stdio_ofl_lockptr` → ...）逐一重置或释放每个锁指针
2. 每个锁 reset 使用 `a_store(lockptr, 0)` 确保原子性
3. 调用 POSIX 标准的 pthread_atfork 注册回调链

**Invariant**: `__post_Fork` 必须保证：返回后，进程内所有全局锁要么被释放（父进程），要么被重置为空闲状态（子进程）。不允许出现任何悬空锁——这将导致死锁。

---

## 全局不变量 (Global Invariants)

1. **锁完整性**: `fork_impl.h` 中列出的每个锁指针在 musl 的可抢占式 fork 实现中都必须被处理。遗漏任何锁指针将导致 fork 后死锁或数据竞争。

2. **回调顺序**: atfork 回调必须按以下顺序执行：
   - Prepare: `__malloc_atfork(-1)`, `__ldso_atfork(-1)`, `__pthread_key_atfork(-1)`
   - Parent: `__pthread_key_atfork(0)`, `__ldso_atfork(0)`, `__malloc_atfork(0)`  
   - Child: `__malloc_atfork(1)`, `__ldso_atfork(1)`, `__pthread_key_atfork(1)`
   
   此顺序确保内层模块（malloc）在外层模块（pthread）之前被处理。

3. **类型安全**: 所有锁指针声明为 `volatile int *const`，确保指针绑定不可变（防止意外重定向），但通过 `volatile` 保证每次访问都从内存读取实际值。