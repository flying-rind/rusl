# libc.h 规约

## 概述

`libc.h` 是 musl 最核心的内部头文件，定义了整个 C 库运行时的全局状态结构体 `struct __libc`、线程局部存储模块描述符 `struct tls_module`、区域设置结构体 `struct __locale_struct`，以及众多初始化和管理函数。该头文件通过 `#define libc __libc` 宏简写，使 musl 所有源文件都能以 `libc.xxx` 的形式直接访问运行时全局状态。

## 依赖图

```
libc.h
├── <stdlib.h>          (标准库, 提供 size_t 等)
├── <stdio.h>           (标准库)
├── <limits.h>          (标准库)
│
├── [结构体定义]
│   ├── struct __locale_map   (前向声明, 定义于 locale_impl.h)
│   ├── struct __locale_struct (定义于本文件, 依赖 __locale_map)
│   ├── struct tls_module     (定义于本文件)
│   └── struct __libc         (定义于本文件, 依赖 tls_module, __locale_struct)
│
├── [全局变量]
│   ├── struct __libc __libc             → 定义于 src/internal/libc.c
│   ├── size_t __hwcap                  → 定义于 src/internal/libc.c
│   ├── size_t __sysinfo               → 定义于 src/internal/defsysinfo.c
│   ├── char *__progname, *__progname_full → 定义于 src/internal/libc.c
│   └── const char __libc_version[]     → 定义于 src/internal/version.c
│
├── [初始化函数]
│   ├── __init_libc(char**, char*)       → 定义于 src/env/__init_libc.c
│   ├── __init_tls(size_t*)             → 定义于 src/env/__init_tls.c
│   ├── __init_ssp(void*)               → 定义于 src/env/__init_ssp.c
│   └── __libc_start_init(void)          → 定义于 src/env/__libc_start_init.c
│
├── [退出/atexit 函数]
│   ├── __funcs_on_exit(void)            → 定义于 src/exit/exit.c
│   ├── __funcs_on_quick_exit(void)      → 定义于 src/exit/quick_exit.c
│   ├── __libc_exit_fini(void)           → 定义于 src/exit/exit.c
│   └── __fork_handler(int)             → 定义于 src/process/fork.c
│
└── [同步/进程管理函数]
    ├── __synccall(void(*)(void*), void*) → 定义于 src/thread/__synccall.c
    └── __setxid(int,int,int,int)        → 定义于 src/thread/__setxid.c
```

## 类型/结构依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `size_t` | `<stdlib.h>` → `<stddef.h>` | C 标准类型，跳过 |
| `struct __locale_map` | `locale_impl.h` | 跨文件依赖，见 locale_impl.h spec |

---

## 结构体规约（按拓扑顺序，从叶子到根）

### struct __locale_map (前向声明)

```c
struct __locale_map;
```

[Visibility]: Internal (不导出) — musl 内部 locale 数据映射结构，完整定义见 `locale_impl.h`

**Intent**: 表示一个加载到内存中的 locale 类别数据（如 `LC_CTYPE`、`LC_COLLATE` 等）。`struct __locale_struct` 通过指针数组引用 6 个类别的 `__locale_map` 实例。

---

### struct __locale_struct

```c
struct __locale_struct {
    const struct __locale_map *cat[6];
};
```
```rust
#[repr(C)]
pub struct LocaleStruct {
    pub cat: [*const LocaleMap; 6],
}
```

[Visibility]: Internal (不导出) — musl 内部 locale 状态，对应 POSIX 的 `locale_t`

**字段契约**:
- `cat[0]` (`LC_CTYPE`): 字符分类与大小写转换表
- `cat[1]` (`LC_NUMERIC`): 数字格式化规则
- `cat[2]` (`LC_TIME`): 时间日期格式化
- `cat[3]` (`LC_COLLATE`): 字符串排序规则
- `cat[4]` (`LC_MONETARY`): 货币格式化
- `cat[5]` (`LC_MESSAGES`): 消息/提示语言

**不变量**: 每个非空 `cat[i]` 指向的 `__locale_map` 必须已通过 `mmap` 加载并验证完整性。

---

### struct tls_module

```c
struct tls_module {
    struct tls_module *next;
    void *image;
    size_t len, size, align, offset;
};
```
```rust
#[repr(C)]
pub struct TlsModule {
    pub next: *mut TlsModule,
    pub image: *mut c_void,
    pub len: usize,
    pub size: usize,
    pub align: usize,
    pub offset: usize,
}
```

[Visibility]: Internal (不导出) — musl 内部 TLS (Thread-Local Storage) 模块描述符

**字段契约**:

| 字段 | 含义 |
|------|------|
| `next` | 指向下一个 TLS 模块的单链表指针 |
| `image` | 指向 ELF 中 TLS 模板数据的指针（初始化镜像） |
| `len` | TLS 模板数据的长度（字节） |
| `size` | 该模块在 TLS 块中的总大小（含对齐填充，`.tbss` 清零区域） |
| `align` | TLS 块的对齐要求 |
| `offset` | 该模块的 TLS 数据在线程 TLS 块中的起始偏移量 |

**不变量**:
1. `offset` 在主线程和所有子线程中相同（由 `__init_tls` 统一计算，保证一致性）。
2. `size >= len`（`.tbss` 未初始化数据部分由 `size - len` 给出）。
3. `align` 必须是 2 的幂。

**Intent**: 每个加载的共享库（含主程序）如果有 TLS 变量，都对应一个 `tls_module` 节点。musl 通过链表管理所有模块，在创建新线程时按相同布局分配 TLS 块。

---

### struct __libc

```c
struct __libc {
    char can_do_threads;
    char threaded;
    char secure;
    volatile signed char need_locks;
    int threads_minus_1;
    size_t *auxv;
    struct tls_module *tls_head;
    size_t tls_size, tls_align, tls_cnt;
    size_t page_size;
    struct __locale_struct global_locale;
};
```
```rust
#[repr(C)]
pub struct Libc {
    pub can_do_threads: c_char,
    pub threaded: c_char,
    pub secure: c_char,
    pub need_locks: c_char,  // AtomicI8 in practice
    pub threads_minus_1: c_int,
    pub auxv: *mut usize,
    pub tls_head: *mut TlsModule,
    pub tls_size: usize,
    pub tls_align: usize,
    pub tls_cnt: usize,
    pub page_size: usize,
    pub global_locale: LocaleStruct,
}
```

[Visibility]: Internal (不导出) — musl 运行时的**唯一全局状态根对象**。每个 musl 进程只有一个 `struct __libc` 实例（定义在 `src/internal/libc.c`）。通过 `#define libc __libc` 宏，所有 musl 内部代码以 `libc.xxx` 形式直接访问。

---

#### 字段契约 — 线程控制

| 字段 | 类型 | 契约 |
|------|------|------|
| `can_do_threads` | `char` | 标记进程是否具有线程能力。`1` = 可以创建线程（链接了 `libpthread`），`0` = 单线程模式 |
| `threaded` | `char` | 标记进程是否已经是多线程的。`1` = 有第二个线程已被创建，`0` = 仅主线程。影响锁策略：单线程时所有锁为 no-op |
| `need_locks` | `volatile signed char` | 负值表示需要锁。`-1` = 需要锁保护，`0` = 无需（`can_do_threads==0` 时）。volatile 确保每次读取从内存重载 |
| `threads_minus_1` | `int` | 当前活跃线程数减 1。单线程时值为 `0`。`fork()` 后在子进程中重置为 `0` |

**Invariant 1 — 锁策略一致性**:
```
need_locks < 0  ⇔  can_do_threads == 1
```
当 `can_do_threads == 0` 时，所有 `LOCK()/UNLOCK()` 宏展开为空操作。

---

#### 字段契约 — 安全与 AUXV

| 字段 | 类型 | 契约 |
|------|------|------|
| `secure` | `char` | setuid/setgid 安全模式标志。`1` = 进程运行在安全模式下（禁止 `LD_PRELOAD`、`LD_LIBRARY_PATH` 等），由 `AT_SECURE` 辅助向量设置 |
| `auxv` | `size_t *` | 指向原始 ELF auxiliary vector 数组的指针（`AT_NULL` 终止）。内核在进程启动时传递 |

---

#### 字段契约 — TLS 管理

| 字段 | 类型 | 契约 |
|------|------|------|
| `tls_head` | `struct tls_module *` | TLS 模块单链表头指针。`NULL` = 无 TLS 模块 |
| `tls_size` | `size_t` | 单个线程所需的 TLS 块总大小（所有模块 `size` 之和，含对齐） |
| `tls_align` | `size_t` | TLS 块的整体对齐要求（所有模块 `align` 的最大公倍数） |
| `tls_cnt` | `size_t` | TLS 模块计数（链表长度） |

---

#### 字段契约 — 系统与区域

| 字段 | 类型 | 契约 |
|------|------|------|
| `page_size` | `size_t` | 系统页大小（字节）。由 `__init_libc` 从 `AT_PAGESZ` 辅助向量获取，用于内存分配和 mmap 对齐 |
| `global_locale` | `struct __locale_struct` | 全局 locale 状态。初始值为 C locale（所有 `cat[i] == NULL`），通过 `setlocale()` 修改 |

---

#### 全局不变量

2. **单例性**: 每个进程只有唯一一个 `struct __libc` 实例。不存在两个 `__libc` 对象。
3. **初始化时序**: `libc` 的字段由 `__init_libc` / `__init_tls` 在 `__libc_start_main` 之前初始化完成。
4. **TLS 一致性**: `tls_size`, `tls_align`, `tls_cnt` 在 `__init_tls` 完成后不可变（除非通过 `dlopen` 加载新库，此时动态链接器更新它们）。

---

## 宏规约

### PAGE_SIZE

```c
#ifndef PAGE_SIZE
#define PAGE_SIZE libc.page_size
#endif
```

[Visibility]: Internal (不导出) — musl 内部使用的页大小宏，解析为运行时全局变量

**Intent**: 允许 musl 代码以 `PAGE_SIZE` 的形式使用系统页大小，该值在运行时从 `AT_PAGESZ` 辅助向量获取。若编译环境已定义 `PAGE_SIZE`（如静态已知），则使用编译期常量。

**不变量**: `PAGE_SIZE` 的值在整个进程生命周期内不变。

---

## 全局变量规约

### __libc

```c
extern hidden struct __libc __libc;
#define libc __libc
```
```rust
// Rust — 全局可变状态
static mut LIBC: Libc = Libc::new();
```

[Visibility]: Internal (不导出) — musl 运行时核心全局状态

**定义位置**: `src/internal/libc.c`

**前置条件（访问前）**: 进程已通过 `__init_libc` / `__init_tls` 完成基本初始化。

**后置条件（初始化后）**: 所有字段被赋予合理默认值或系统特定值。

---

### __hwcap

```c
extern hidden size_t __hwcap;
```

[Visibility]: Internal (不导出) — CPU 硬件能力位掩码（来自 `AT_HWCAP`）

**定义位置**: `src/internal/libc.c`, 由 `__init_libc` 设置。

**Intent**: 存储 CPU 特性位掩码（如 x86 的 SSE/AVX, ARM 的 NEON），musl 内部用于运行时选择优化的汇编例程（如 `memcpy` 的 SIMD 版本）。

---

### __sysinfo

```c
extern hidden size_t __sysinfo;
```

[Visibility]: Internal (不导出) — ARM EABI 系统调用辅助信息地址

**定义位置**: `src/internal/defsysinfo.c`, 由 `AT_SYSINFO_EHDR` 设置。

**Intent**: 仅在 ARM 架构上有意义，存储内核通过 `AT_SYSINFO_EHDR` 辅助向量传递的 VDSO/kuser_helper 地址。x86 上始终为 0。

---

### __progname / __progname_full

```c
extern char *__progname, *__progname_full;
```

[Visibility]: 复合可见性

| 符号 | 可见性 | 说明 |
|------|--------|------|
| `__progname` | Internal | musl 内部变量 |
| `__progname_full` | Internal | musl 内部变量 |
| `program_invocation_short_name` | **Public** (通过 weak_alias) | GNU 扩展，`<errno.h>` 声明 |
| `program_invocation_name` | **Public** (通过 weak_alias) | GNU 扩展，`<errno.h>` 声明 |

**定义位置**: `src/internal/libc.c`, 通过 `weak_alias` 暴露为 GNU 扩展。

**前置条件**: 由 `__init_libc` 从 `argv[0]` 初始化。初始化前值为 `NULL`。

**后置条件**:
- `__progname`: 指向 `argv[0]` 中最后一个 `/` 之后的字符
- `__progname_full`: 指向 `argv[0]` 的完整副本（通过内部分配函数）或 `NULL`

---

### __libc_version

```c
extern hidden const char __libc_version[];
```

[Visibility]: Internal (不导出) — musl 版本字符串

**定义位置**: `src/internal/version.c`

**Intent**: 存储 musl 版本信息字符串（如 `"1.2.5"`），用于 `confstr(_CS_GNU_LIBC_VERSION, ...)` 等查询。

---

## 函数规约（按调用时序排列）

### __init_libc

```c
hidden void __init_libc(char **envp, char *pn);
```
```rust
fn __init_libc(envp: *mut *mut c_char, pn: *mut c_char);
```

[Visibility]: Internal (不导出) — musl 启动早期初始化

**前置条件**:
1. 由 `_start` / `__libc_start_main` 在最早期调用，先于任何 musl 功能使用
2. `envp` 指向环境变量指针数组（以 `NULL` 终止）
3. `pn` 指向 `argv[0]`（程序名）

**后置条件**:
- `libc.page_size` 已从 `AT_PAGESZ` 设置（若未获取到，默认 4096）
- `libc.secure` 已根据 `AT_SECURE` 设置
- `libc.auxv` 指向 auxiliary vector
- `__progname` / `__progname_full` 已从 `pn` 初始化
- `__hwcap` 已从 `AT_HWCAP` 设置
- 环境变量 `__libc.need_locks` 已根据是否有线程能力设置

---

### __init_tls

```c
hidden void __init_tls(size_t *auxv);
```
```rust
fn __init_tls(auxv: *mut usize);
```

[Visibility]: Internal (不导出) — TLS (线程局部存储) 初始化

**前置条件**:
1. 在 `__init_libc` 之后调用
2. `auxv` 指向 auxiliary vector

**后置条件**:
- 主线程的 TLS 块已分配并初始化
- `libc.tls_head` 链表已构建
- `libc.tls_size`, `libc.tls_align`, `libc.tls_cnt` 已计算
- 若架构使用 TLS 寄存器（如 x86 的 `%fs`），该寄存器已设置

---

### __init_ssp

```c
hidden void __init_ssp(void *entropy);
```
```rust
fn __init_ssp(entropy: *mut c_void);
```

[Visibility]: Internal (不导出) — Stack Smashing Protector 初始化

**前置条件**:
1. 在 `__init_libc` 之后调用
2. `entropy` 指向至少 `sizeof(uintptr_t)` 字节的随机数据

**后置条件**:
- `__stack_chk_guard` 已设置为基于 `entropy` 的 canary 值

---

### __libc_start_init

```c
hidden void __libc_start_init(void);
```
```rust
fn __libc_start_init();
```

[Visibility]: Internal (不导出) — 调用所有共享库和主程序的初始化函数

**前置条件**: TLS 已初始化，libc 状态已就绪

**后置条件**:
- 所有共享库的 `.init` / `.init_array` 段中的初始化函数已被调用
- 主程序的 `.init_array` 函数已被调用
- 调用顺序：先依赖库，后主程序

---

### __funcs_on_exit

```c
hidden void __funcs_on_exit(void);
```
```rust
fn __funcs_on_exit();
```

[Visibility]: Internal (不导出) — 执行 `atexit` 注册的所有退出处理函数

**前置条件**: 由 `exit()` 或 `return from main` 调用

**后置条件**:
- 所有通过 `atexit()` 注册的函数按注册的**逆序**被调用
- 所有 stdio 流被刷新（`fflush(NULL)`）

---

### __funcs_on_quick_exit

```c
hidden void __funcs_on_quick_exit(void);
```
```rust
fn __funcs_on_quick_exit();
```

[Visibility]: Internal (不导出) — 执行 `at_quick_exit` 注册的快速退出处理函数

**前置条件**: 由 `quick_exit()` 调用

**后置条件**: 所有通过 `at_quick_exit()` 注册的函数按注册的逆序被调用

---

### __libc_exit_fini

```c
hidden void __libc_exit_fini(void);
```
```rust
fn __libc_exit_fini();
```

[Visibility]: Internal (不导出) — 调用共享库的终止函数

**前置条件**: 由 `exit()` 在 `__funcs_on_exit` 之后调用

**后置条件**: 所有共享库的 `.fini` / `.fini_array` 段中的终止函数被调用（逆序）

---

### __fork_handler

```c
hidden void __fork_handler(int who);
```
```rust
fn __fork_handler(who: c_int);
```

[Visibility]: Internal (不导出) — fork 锁处理的中央调度函数

**前置条件**:
- `who` ∈ {`-1`, `0`, `1`}：prepare / parent / child
- 被 `fork()` 实现调用

**后置条件**: 见 `fork_impl.h` spec —— 该函数协调所有 atfork 回调的执行顺序

**Intent**: `__fork_handler` 是 fork 锁管理的中央调度器，按正确顺序调用 `__malloc_atfork`, `__ldso_atfork`, `__pthread_key_atfork`，以及 `__post_Fork`。

---

### __synccall

```c
hidden void __synccall(void (*func)(void *), void *ctx);
```
```rust
fn __synccall(func: extern "C" fn(*mut c_void), ctx: *mut c_void);
```

[Visibility]: Internal (不导出) — 在所有线程上同步执行指定函数

**前置条件**:
1. 进程是多线程的（`libc.threads_minus_1 >= 1`），或者仅有主线程（此时直接调用）
2. `func` 不可为 `NULL`
3. `ctx` 为传递给 `func` 的不透明上下文指针

**后置条件**:
- `func(ctx)` 已在**每个活跃线程**的上下文中被调用
- 调用期间所有其他线程被暂停（通过信号 `SIGSYNCCALL` 实现）
- 调用者线程也执行 `func(ctx)`

**System Algorithm**: 使用 `SIGSYNCCALL` 信号广播给所有线程，每个线程在信号处理器中执行 `func(ctx)` 并等待所有线程完成（通过屏障同步），然后统一返回。

**Intent**: 实现全局同步操作。例如 `setuid()` 在多线程程序中需要通过 `__synccall` 确保每个线程的 UID 都被设置。

---

### __setxid

```c
hidden int __setxid(int nr, int id, int eid, int sid);
```
```rust
fn __setxid(nr: c_int, id: c_int, eid: c_int, sid: c_int) -> c_int;
```

[Visibility]: Internal (不导出) — 在多线程程序中同步设置 UID/GID

**前置条件**:
1. `nr` 为系统调用号（`SYS_setuid`, `SYS_setgid`, `SYS_setreuid`, `SYS_setregid`, `SYS_setresuid`, `SYS_setresgid` 之一）
2. `id`, `eid`, `sid` 的含义取决于 `nr` 指定的系统调用

**后置条件**:
- 调用线程的凭据已通过系统调用设置
- 若进程是多线程的，通过 `__synccall` 同步到所有线程
- 返回 0 成功，或 `-1` 且设置 `errno`

**Intent**: POSIX 要求 `setuid()` 等函数在多线程程序中设置**所有线程**的 UID，而不仅仅是调用线程。`__setxid` 通过 `__synccall` 确保同步。