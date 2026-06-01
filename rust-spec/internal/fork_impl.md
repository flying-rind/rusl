# fork_impl.rs 规约

## 概述

`fork_impl` 模块声明了 rusl 在多线程环境下 `fork()` 的安全实现所需的所有锁指针和 atfork 回调函数。由于 `fork()` 通过系统调用复制整个进程地址空间，但在多线程程序中，其他线程可能持有锁，子进程中的这些锁将永远无法释放。rusl 的解决方案是：在 fork 前获取所有全局锁，在 fork 后在父子进程中分别释放或重置锁。

## 依赖图

```
fork_impl 模块
├── core::sync::atomic::AtomicI32  (Rust core)
│
├── [锁变量] (声明于本模块，定义于各子模块)
│   ├── AT_QUICK_EXIT_LOCK      → 定义于 exit/at_quick_exit.rs
│   ├── ATEXIT_LOCK             → 定义于 exit/atexit.rs
│   ├── GETTEXT_LOCK            → 定义于 locale/gettext.rs (若存在)
│   ├── LOCALE_LOCK             → 定义于 locale/locale_map.rs
│   ├── RANDOM_LOCK             → 定义于 prng/random.rs
│   ├── SEM_OPEN_LOCK           → 定义于 sem_open.rs
│   ├── STDIO_OFL_LOCK          → 定义于 stdio/ofl.rs
│   ├── SYSLOG_LOCK             → 定义于 misc/syslog.rs
│   ├── TIMEZONE_LOCK           → 定义于 time/tz.rs
│   ├── BUMP_LOCK               → 定义于 malloc/malloc.rs (bump allocator)
│   └── VMLOCK_LOCK             → 定义于 mmap 相关
│
├── [Atfork 回调] (声明于本模块，定义于各模块)
│   ├── malloc_atfork(who: c_int)      → 定义于 malloc/malloc.rs
│   ├── ldso_atfork(who: c_int)        → 定义于 ldso/dynlink.rs
│   └── pthread_key_atfork(who: c_int) → 定义于 thread/pthread_key_create.rs
│
└── [Fork 后处理]
    └── post_Fork(ret: c_int)          → 定义于 process/fork.rs
```

---

## 类型规约

### 锁类型

```rust
// Rust 声明 (rusl)
// 每个锁是模块私有的 AtomicI32 静态变量
// 其地址通过 pub(crate) 函数暴露给 fork_impl 模块进行收集
//
// C 等价: volatile int *const
// Rust: &'static AtomicI32 （指针不可变，指向的值可原子操作）
```

[Visibility]: Internal — rusl 内部锁机制，非 POSIX 标准定义

**Intent**: 每个锁是一个 `AtomicI32` 类型的 futex 字，用作该模块的全局锁。在 Rust 中，使用 `AtomicI32` 替代 C 的 `volatile int`，提供类型安全的原子操作。锁指针不可变（`&'static`），指向的值在持有锁时为持有线程的 TID，锁空闲时为 0。

---

## 符号规约

### 锁变量组 (11 个全局锁)

每个锁变量遵循以下统一模式：

- **类型**: `AtomicI32`，通过静态变量持有，使用 `pub(crate)` 暴露地址引用
- **锁协议**: `0` = 空闲，`非零` = 由该 TID 持有
- **fork 行为**: 
  - fork 前（prepare）: 逐个获取（原子 CAS 操作）
  - 父进程（parent）: 逐个释放（原子 store(0)）
  - 子进程（child）: 逐个重置（原子 store(0)）

---

#### `AT_QUICK_EXIT_LOCK`

```rust
// Rust 声明 (rusl) — 定义于 exit/at_quick_exit.rs
pub(crate) static AT_QUICK_EXIT_LOCK: AtomicI32 = AtomicI32::new(0);
```

```c
// C 等价声明 (musl)
extern hidden volatile int *const __at_quick_exit_lockptr;
```

[Visibility]: Internal — rusl 内部锁，保护 `at_quick_exit` 处理函数链表的并发访问

**Intent**: 保护 `quick_exit` / `at_quick_exit` 函数注册链表的并发修改。

---

#### `ATEXIT_LOCK`

```rust
// Rust 声明 (rusl) — 定义于 exit/atexit.rs
pub(crate) static ATEXIT_LOCK: AtomicI32 = AtomicI32::new(0);
```

```c
// C 等价声明 (musl)
extern hidden volatile int *const __atexit_lockptr;
```

[Visibility]: Internal — rusl 内部锁，保护 `atexit` 处理函数链表

**Intent**: 保护 `exit` / `atexit` 函数注册链表的并发修改。

---

#### `GETTEXT_LOCK`

```rust
// Rust 声明 (rusl) — 定义于 locale/gettext.rs
pub(crate) static GETTEXT_LOCK: AtomicI32 = AtomicI32::new(0);
```

```c
// C 等价声明 (musl)
extern hidden volatile int *const __gettext_lockptr;
```

[Visibility]: Internal — rusl 内部锁，保护 gettext 国际化数据

**Intent**: 保护 gettext / 国际化消息目录的并发加载与访问。

---

#### `LOCALE_LOCK`

```rust
// Rust 声明 (rusl) — 定义于 locale/locale_map.rs
pub(crate) static LOCALE_LOCK: AtomicI32 = AtomicI32::new(0);
```

```c
// C 等价声明 (musl)
extern hidden volatile int *const __locale_lockptr;
```

[Visibility]: Internal — rusl 内部锁，保护 locale 数据结构

**Intent**: 保护 locale 结构体的并发修改（`setlocale` 调用等）。

---

#### `RANDOM_LOCK`

```rust
// Rust 声明 (rusl) — 定义于 prng/random.rs
pub(crate) static RANDOM_LOCK: AtomicI32 = AtomicI32::new(0);
```

```c
// C 等价声明 (musl)
extern hidden volatile int *const __random_lockptr;
```

[Visibility]: Internal — rusl 内部锁，保护随机数生成器状态

**Intent**: 保护 `random()` / `srandom()` 内部 PRNG 状态的并发访问。

---

#### `SEM_OPEN_LOCK`

```rust
// Rust 声明 (rusl) — 定义于 sem_open.rs
pub(crate) static SEM_OPEN_LOCK: AtomicI32 = AtomicI32::new(0);
```

```c
// C 等价声明 (musl)
extern hidden volatile int *const __sem_open_lockptr;
```

[Visibility]: Internal — rusl 内部锁，保护命名信号量全局列表

**Intent**: 保护 `sem_open` / `sem_close` / `sem_unlink` 中命名信号量全局注册表。

---

#### `STDIO_OFL_LOCK`

```rust
// Rust 声明 (rusl) — 定义于 stdio/ofl.rs
pub(crate) static STDIO_OFL_LOCK: AtomicI32 = AtomicI32::new(0);
```

```c
// C 等价声明 (musl)
extern hidden volatile int *const __stdio_ofl_lockptr;
```

[Visibility]: Internal — rusl 内部锁，保护 stdio 打开文件列表

**Intent**: 保护 `FILE` 结构体打开的全局文件链表，在 `fopen` / `fclose` 中用于添加/移除文件。

---

#### `SYSLOG_LOCK`

```rust
// Rust 声明 (rusl) — 定义于 misc/syslog.rs
pub(crate) static SYSLOG_LOCK: AtomicI32 = AtomicI32::new(0);
```

```c
// C 等价声明 (musl)
extern hidden volatile int *const __syslog_lockptr;
```

[Visibility]: Internal — rusl 内部锁，保护 syslog 连接

**Intent**: 保护 `syslog()` 内部使用的 Unix 域套接字连接状态。

---

#### `TIMEZONE_LOCK`

```rust
// Rust 声明 (rusl) — 定义于 time/tz.rs
pub(crate) static TIMEZONE_LOCK: AtomicI32 = AtomicI32::new(0);
```

```c
// C 等价声明 (musl)
extern hidden volatile int *const __timezone_lockptr;
```

[Visibility]: Internal — rusl 内部锁，保护时区数据

**Intent**: 保护时区相关全局变量的并发访问。

---

#### `BUMP_LOCK`

```rust
// Rust 声明 (rusl) — 定义于 malloc/malloc.rs
pub(crate) static BUMP_LOCK: AtomicI32 = AtomicI32::new(0);
```

```c
// C 等价声明 (musl)
extern hidden volatile int *const __bump_lockptr;
```

[Visibility]: Internal — rusl 内部锁，保护 bump allocator

**Intent**: 保护 rusl 内部使用的线性 bump 分配器，用于分配库内部的小块内存。

---

#### `VMLOCK_LOCK`

```rust
// Rust 声明 (rusl) — 定义于 mmap 相关
pub(crate) static VMLOCK_LOCK: AtomicI32 = AtomicI32::new(0);
```

```c
// C 等价声明 (musl)
extern hidden volatile int *const __vmlock_lockptr;
```

[Visibility]: Internal — rusl 内部锁，保护虚拟内存操作

**Intent**: 保护 `mmap` / `munmap` 相关内部状态，防止 fork 时内存映射状态不一致。

---

### Atfork 回调函数

#### `malloc_atfork`

```rust
// Rust 声明 (rusl)
pub(crate) fn malloc_atfork(who: c_int);
```

```c
// C 等价声明 (musl)
hidden void __malloc_atfork(int who);
```

[Visibility]: Internal — rusl 内部 atfork 回调，由 `fork()` 实现调用

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

#### `ldso_atfork`

```rust
// Rust 声明 (rusl)
pub(crate) fn ldso_atfork(who: c_int);
```

```c
// C 等价声明 (musl)
hidden void __ldso_atfork(int who);
```

[Visibility]: Internal — rusl 内部 atfork 回调，由动态链接器提供

**前置条件**:
- `who` ∈ {`-1`, `0`, `1`}
- 调用者必须处于 `fork()` 的关键区段

**后置条件**:
- `who == -1`: 获取动态链接器内部锁
- `who == 0`: 释放动态链接器锁
- `who == 1`: 重置动态链接器锁（子进程中无其他线程）

**Intent**: 保护动态链接器的内部数据结构（如加载的共享库链表）在 fork 期间的一致性。

---

#### `pthread_key_atfork`

```rust
// Rust 声明 (rusl)
pub(crate) fn pthread_key_atfork(who: c_int);
```

```c
// C 等价声明 (musl)
hidden void __pthread_key_atfork(int who);
```

[Visibility]: Internal — rusl 内部 atfork 回调，由 pthread 实现提供

**前置条件**:
- `who` ∈ {`-1`, `0`, `1`}
- 仅在多线程程序中有效（`thread_count > 1`）

**后置条件**:
- `who == -1`: 获取 pthread key 内部锁
- `who == 0`: 释放 pthread key 锁
- `who == 1`: 重置 pthread key 锁，重置 TSD (Thread-Specific Data) 析构链表

**Intent**: 保护 pthread 线程特定数据 (TSD / TLS key) 在 fork 后的正确性。

---

### Fork 后处理

#### `post_Fork`

```rust
// Rust 声明 (rusl)
pub(crate) fn post_Fork(ret: c_int);
```

```c
// C 等价声明 (musl)
hidden void __post_Fork(int ret);
```

[Visibility]: Internal — rusl 内部，`fork()` 系统调用返回后立即调用

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
  - 重置 libc 线程计数器为 `0`
  - 调用所有 atfork 子进程回调
  - 清理其他继承自父进程的状态（如 robust mutex 列表）
- **Case 3 — fork 失败 (`ret < 0`)**:
  - 释放 prepare 阶段获取的所有锁
  - 设置 `errno` 并返回

**System Algorithm**: 该函数按固定顺序处理：
1. 按持有锁的逆序（`BUMP_LOCK` → `VMLOCK_LOCK` → `STDIO_OFL_LOCK` → ...）逐一重置或释放每个锁
2. 每个锁 reset 使用 `.store(0, Ordering::Release)` 确保原子性和内存可见性
3. 调用 POSIX 标准的 pthread_atfork 注册回调链

**Invariant**: `post_Fork` 必须保证：返回后，进程内所有全局锁要么被释放（父进程），要么被重置为空闲状态（子进程）。不允许出现任何悬空锁——这将导致死锁。

---

## 全局不变量 (Global Invariants)

1. **锁完整性**: `fork_impl` 模块中列出的每个锁在 rusl 的可抢占式 fork 实现中都必须被处理。遗漏任何锁将导致 fork 后死锁或数据竞争。

2. **回调顺序**: atfork 回调必须按以下顺序执行：
   - Prepare: `malloc_atfork(-1)`, `ldso_atfork(-1)`, `pthread_key_atfork(-1)`
   - Parent: `pthread_key_atfork(0)`, `ldso_atfork(0)`, `malloc_atfork(0)`
   - Child: `malloc_atfork(1)`, `ldso_atfork(1)`, `pthread_key_atfork(1)`

   此顺序确保内层模块（malloc）在外层模块（pthread）之前被处理。

3. **类型安全**: 所有锁使用 Rust 的 `AtomicI32` 类型，通过 `.load()` / `.store()` / `.compare_exchange()` 等原子操作访问，摒弃 C 的 `volatile int` 手工同步。

---

## Rust 实现注意事项 (`#![no_std]`)

1. **原子类型**: 使用 `core::sync::atomic::AtomicI32` 替代 C 的 `volatile int`，提供类型安全且无数据竞争的锁实现。
2. **unsafe 使用**: 锁的获取/释放操作完全是安全的（`AtomicI32` 的操作均在安全 Rust 中可用）。`unsafe` 仅用于：
   - 实际的 `fork()` 系统调用（裸系统调用）
   - 在子进程中重置 pthread 内部状态时可能需要的某些操作
3. **锁收集**: 锁不通过指针暴露，而是通过 `fork_impl` 模块维护一个 `&'static AtomicI32` 引用数组来收集所有锁，便于 `post_Fork` 统一处理。
4. **回调注册**: atfork 回调函数可以设计为函数指针类型，通过 `static` 变量注册，便于 `fork_impl` 模块统一调用。