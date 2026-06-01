# libc 规约 (Rust)

## 概述

`libc` 模块是 rusl 最核心的内部模块，定义了整个 C 库运行时的全局状态结构体 `Libc`、线程局部存储模块描述符 `TlsModule`、区域设置结构体 `LocaleStruct`，以及众多初始化和管理函数。该模块通过一个全局静态变量使 rusl 所有源文件都能访问运行时全局状态。

## 依赖图

```
libc 模块
├── [结构体定义]
│   ├── struct LocaleMap       (前向声明, 定义于 locale_impl 模块)
│   ├── struct LocaleStruct    (定义于本模块, 依赖 LocaleMap)
│   ├── struct TlsModule       (定义于本模块)
│   └── struct Libc            (定义于本模块, 依赖 TlsModule, LocaleStruct)
│
├── [全局变量]
│   ├── static mut LIBC: Libc                  → 定义于 libc 模块
│   ├── static mut __hwcap: usize              → 定义于 libc 模块
│   ├── static mut __sysinfo: usize            → 定义于 defsysinfo 模块
│   ├── static mut __progname: *mut c_char     → 定义于 libc 模块
│   ├── static mut __progname_full: *mut c_char → 定义于 libc 模块
│   └── static __libc_version: &'static [u8]   → 定义于 version 模块
│
├── [初始化函数]
│   ├── __init_libc(envp, pn)       → 定义于 env/__init_libc 模块
│   ├── __init_tls(auxv)            → 定义于 env/__init_tls 模块
│   ├── __init_ssp(entropy)         → 定义于 env/__init_ssp 模块
│   └── __libc_start_init()         → 定义于 env/__libc_start_init 模块
│
├── [退出/atexit 函数]
│   ├── __funcs_on_exit()           → 定义于 exit 模块
│   ├── __funcs_on_quick_exit()     → 定义于 exit 模块
│   ├── __libc_exit_fini()          → 定义于 exit 模块
│   └── __fork_handler(who)         → 定义于 process/fork 模块
│
└── [同步/进程管理函数]
    ├── __synccall(func, ctx)       → 定义于 thread/__synccall 模块
    └── __setxid(nr, id, eid, sid)  → 定义于 thread/__setxid 模块
```

---

```
/* Rely */
[RELY]
结构体依赖:
  struct LocaleMap { ... };                    // 依赖1: locale 数据映射结构(定义于 locale_impl 模块)
内部依赖:
  struct pthread { ... };                      // 依赖2: 线程控制块(定义于 pthread_impl 模块)
  fn __pthread_self() -> *mut pthread;         // 依赖3: 获取当前线程控制块
系统依赖:
  AT_PAGESZ, AT_SECURE, AT_HWCAP, AT_SYSINFO_EHDR 等辅助向量常量  // 依赖4: ELF auxiliary vector
内部依赖:
  __stack_chk_guard (定义于 ssp 模块)          // 依赖5: SSP canary 值
  __malloc_atfork / __ldso_atfork / __pthread_key_atfork / __post_Fork
                                                // 依赖6: fork 回调函数

[GUARANTEE]
外部接口(public, C ABI 兼容):
  extern "C" fn program_invocation_short_name;  // GNU 扩展, 声明于 <errno.h>
  extern "C" fn program_invocation_name;        // GNU 扩展, 声明于 <errno.h>

内部接口:
  #[repr(C)] pub(crate) struct Libc;            // 本模块保证: 运行时唯一全局状态根对象
  #[repr(C)] pub(crate) struct TlsModule;       // 本模块保证: TLS 模块描述符
  #[repr(C)] pub(crate) struct LocaleStruct;    // 本模块保证: locale 状态结构体
  pub(crate) static mut LIBC: Libc;             // 本模块保证: 全局单例
  pub(crate) fn __init_libc(...);               // 本模块保证: 初始化流程按序执行
  ...
```

---

## 结构体规约（按拓扑顺序，从叶子到根）

### LocaleStruct

```rust
// Rust — 区域设置状态结构体, 对应 POSIX 的 locale_t
#[repr(C)]
pub(crate) struct LocaleStruct {
    pub cat: [*const LocaleMap; 6],  // 6 个 LC_* 类别的 locale 数据映射指针
}
```

[Visibility]: Internal (不导出) — rusl 内部 locale 状态

**字段契约**:
- `cat[0]` (`LC_CTYPE`): 字符分类与大小写转换表
- `cat[1]` (`LC_NUMERIC`): 数字格式化规则
- `cat[2]` (`LC_TIME`): 时间日期格式化
- `cat[3]` (`LC_COLLATE`): 字符串排序规则
- `cat[4]` (`LC_MONETARY`): 货币格式化
- `cat[5]` (`LC_MESSAGES`): 消息/提示语言

**不变量**: 每个非空 `cat[i]` 指向的 `LocaleMap` 必须已通过 `mmap` 加载并验证完整性。`null` 指针表示使用 C locale 默认值。

---

### TlsModule

```rust
// Rust — 线程局部存储 (TLS) 模块描述符
#[repr(C)]
pub(crate) struct TlsModule {
    pub next: *mut TlsModule,   // 单链表: 指向下一个 TLS 模块
    pub image: *mut c_void,     // ELF 中 TLS 模板数据的指针(初始化镜像)
    pub len: usize,             // TLS 模板数据的长度(字节)
    pub size: usize,            // 该模块在 TLS 块中的总大小(含对齐填充 + .tbss 清零区域)
    pub align: usize,           // TLS 块的对齐要求(必须为2的幂)
    pub offset: usize,          // 该模块的 TLS 数据在线程 TLS 块中的起始偏移量
}
```

[Visibility]: Internal (不导出) — rusl 内部 TLS 模块描述符

**字段契约**:

| 字段 | 含义 |
|------|------|
| `next` | 指向下一个 TLS 模块的单链表指针 |
| `image` | 指向 ELF 中 TLS 模板数据的指针(初始化镜像) |
| `len` | TLS 模板数据的长度(字节) |
| `size` | 该模块在 TLS 块中的总大小(含对齐填充，`.tbss` 清零区域) |
| `align` | TLS 块的对齐要求 |
| `offset` | 该模块的 TLS 数据在线程 TLS 块中的起始偏移量 |

**不变量**:
1. `offset` 在主线程和所有子线程中相同(由 `__init_tls` 统一计算，保证一致性)。
2. `size >= len`(`.tbss` 未初始化数据部分由 `size - len` 给出)。
3. `align` 必须是 2 的幂。

**Intent**: 每个加载的共享库(含主程序)如果有 TLS 变量，都对应一个 `TlsModule` 节点。rusl 通过链表管理所有模块，在创建新线程时按相同布局分配 TLS 块。

---

### Libc

```rust
// Rust — rusl 运行时核心全局状态(唯一单例)
#[repr(C)]
pub(crate) struct Libc {
    pub can_do_threads: c_char,      // 进程是否具有线程能力(链接了 libpthread)
    pub threaded: c_char,            // 进程是否已经多线程
    pub secure: c_char,              // setuid/setgid 安全模式标志(来自 AT_SECURE)
    pub need_locks: c_char,          // 是否需要锁保护(负值=true, 0=false)
    pub threads_minus_1: c_int,      // 当前活跃线程数减1
    pub auxv: *mut usize,            // 原始 ELF auxiliary vector 数组(AT_NULL 终止)
    pub tls_head: *mut TlsModule,    // TLS 模块单链表头指针
    pub tls_size: usize,             // 单线程 TLS 块总大小
    pub tls_align: usize,             // TLS 块整体对齐
    pub tls_cnt: usize,              // TLS 模块计数
    pub page_size: usize,            // 系统页大小(字节)
    pub global_locale: LocaleStruct, // 全局 locale 状态
}
```

[Visibility]: Internal (不导出) — rusl 运行时的**唯一全局状态根对象**。每个 rusl 进程只有一个 `Libc` 实例(定义在 `libc` 模块中)，通过 `lazy_static` 或 `static mut` 访问。

---

#### 字段契约 — 线程控制

| 字段 | 类型 | 契约 |
|------|------|------|
| `can_do_threads` | `c_char` | 标记进程是否具有线程能力。1 = 可以创建线程(链接了 `libpthread`)，0 = 单线程模式 |
| `threaded` | `c_char` | 标记进程是否已经是多线程的。1 = 有第二个线程已被创建，0 = 仅主线程。影响锁策略: 单线程时所有锁为 no-op |
| `need_locks` | `c_char` | 负值表示需要锁。-1 = 需要锁保护，0 = 无需(`can_do_threads == 0` 时)。在 Rust 中访问时需使用 `core::sync::atomic::fence` 或 volatile 语义确保重载 |
| `threads_minus_1` | `c_int` | 当前活跃线程数减 1。单线程时值为 0。`fork()` 后在子进程中重置为 0 |

**Invariant 1 — 锁策略一致性**:
```
need_locks < 0  <==>  can_do_threads == 1
```
当 `can_do_threads == 0` 时，所有 `LOCK()/UNLOCK()` 宏展开为空操作。

---

#### 字段契约 — 安全与 AUXV

| 字段 | 类型 | 契约 |
|------|------|------|
| `secure` | `c_char` | setuid/setgid 安全模式标志。1 = 进程运行在安全模式下(禁止 `LD_PRELOAD`、`LD_LIBRARY_PATH` 等)，由 `AT_SECURE` 辅助向量设置 |
| `auxv` | `*mut usize` | 指向原始 ELF auxiliary vector 数组的指针(`AT_NULL` 终止)。内核在进程启动时传递 |

---

#### 字段契约 — TLS 管理

| 字段 | 类型 | 契约 |
|------|------|------|
| `tls_head` | `*mut TlsModule` | TLS 模块单链表头指针。`null` = 无 TLS 模块 |
| `tls_size` | `usize` | 单个线程所需的 TLS 块总大小(所有模块 `size` 之和，含对齐) |
| `tls_align` | `usize` | TLS 块的整体对齐要求(所有模块 `align` 的最大公倍数) |
| `tls_cnt` | `usize` | TLS 模块计数(链表长度) |

---

#### 字段契约 — 系统与区域

| 字段 | 类型 | 契约 |
|------|------|------|
| `page_size` | `usize` | 系统页大小(字节)。由 `__init_libc` 从 `AT_PAGESZ` 辅助向量获取，用于内存分配和 mmap 对齐 |
| `global_locale` | `LocaleStruct` | 全局 locale 状态。初始值为 C locale(所有 `cat[i]` 为 null)，通过 `setlocale()` 修改 |

---

#### 全局不变量

1. **单例性**: 每个进程只有唯一一个 `Libc` 实例。不存在两个 `Libc` 对象。
2. **初始化时序**: `Libc` 的字段由 `__init_libc` / `__init_tls` 在 `__libc_start_main` 之前初始化完成。
3. **TLS 一致性**: `tls_size`, `tls_align`, `tls_cnt` 在 `__init_tls` 完成后不可变(除非通过 `dlopen` 加载新库，此时动态链接器更新它们)。
4. **need_locks 的内存顺序**: 读取 `need_locks` 时需要使用 `Acquire` 顺序的原子操作或编译器屏障，以确保其值与 `can_do_threads` / `threaded` 的状态一致。

---

## 全局变量规约

### LIBC (全局状态根对象)

```rust
// Rust — 全局可变状态(唯一单例)
#[no_mangle]
static mut LIBC: Libc = Libc::new();  // Libc::new() 提供零初始化或默认值
```

[Visibility]: Internal (不导出) — rusl 运行时核心全局状态

**定义位置**: `src/internal/libc.rs`

**前置条件(访问前)**: 进程已通过 `__init_libc` / `__init_tls` 完成基本初始化。

**后置条件(初始化后)**: 所有字段被赋予合理默认值或系统特定值。

---

### __hwcap

```rust
// Rust — CPU 硬件能力位掩码(来自 AT_HWCAP)
static mut __hwcap: usize = 0;
```

[Visibility]: Internal (不导出) — CPU 硬件能力位掩码(来自 `AT_HWCAP`)

**Intent**: 存储 CPU 特性位掩码(如 x86 的 SSE/AVX, ARM 的 NEON)，rusl 内部用于运行时选择优化的汇编例程(如 `memcpy` 的 SIMD 版本)。

---

### __sysinfo

```rust
// Rust — ARM EABI 系统调用辅助信息地址
static mut __sysinfo: usize = 0;
```

[Visibility]: Internal (不导出) — ARM EABI 系统调用辅助信息地址

**Intent**: 仅在 ARM 架构上有意义，存储内核通过 `AT_SYSINFO_EHDR` 辅助向量传递的 VDSO/kuser_helper 地址。x86 上始终为 0。

---

### __progname / __progname_full

```rust
// Rust — 程序名(内部变量 + GNU 扩展公开导出)
static mut __progname: *mut c_char = null_mut();
static mut __progname_full: *mut c_char = null_mut();

// 对外导出 (C ABI 兼容):
#[no_mangle]
pub static mut program_invocation_short_name: *mut c_char;  // = &__progname
#[no_mangle]
pub static mut program_invocation_name: *mut c_char;         // = &__progname_full
```

[Visibility]: 复合可见性

| 符号 | 可见性 | 说明 |
|------|--------|------|
| `__progname` | Internal | rusl 内部变量 |
| `__progname_full` | Internal | rusl 内部变量 |
| `program_invocation_short_name` | **Public**(extern "C") | GNU 扩展, `<errno.h>` 声明 |
| `program_invocation_name` | **Public**(extern "C") | GNU 扩展, `<errno.h>` 声明 |

**前置条件**: 由 `__init_libc` 从 `argv[0]` 初始化。初始化前值为 `null`。

**后置条件**:
- `__progname`: 指向 `argv[0]` 中最后一个 `/` 之后的字符
- `__progname_full`: 指向 `argv[0]` 的完整副本(通过内部分配函数)或 `null`

**ABI 注意**: `program_invocation_short_name` 和 `program_invocation_name` 是 GNU 扩展，必须在 extern "C" 下以正确的 C ABI 导出。在 C 实现中它们通过 `weak_alias` 绑定到 `__progname` 和 `__progname_full`。Rust 中可使用 `#[no_mangle]` + `#[link_name]` 或直接导出同一地址。

---

### __libc_version

```rust
// Rust — musl/rusl 版本字符串
static __libc_version: &[u8] = b"1.2.5\0";
```

[Visibility]: Internal (不导出) — rusl 版本字符串

**Intent**: 存储 rusl 版本信息字符串(如 `"1.2.5"`)，用于 `confstr(_CS_GNU_LIBC_VERSION, ...)` 等查询。

---

## 函数规约（按调用时序排列）

### __init_libc

```rust
// Rust — 启动早期初始化
fn __init_libc(envp: *mut *mut c_char, pn: *mut c_char);
```

[Visibility]: Internal (不导出) — rusl 启动早期初始化

**前置条件**:
1. 由 `_start` / `__libc_start_main` 在最早期调用，先于任何 rusl 功能使用
2. `envp` 指向环境变量指针数组(以 `null` 终止)
3. `pn` 指向 `argv[0]`(程序名)

**后置条件**:
- `LIBC.page_size` 已从 `AT_PAGESZ` 设置(若未获取到，默认 4096)
- `LIBC.secure` 已根据 `AT_SECURE` 设置
- `LIBC.auxv` 指向 auxiliary vector
- `__progname` / `__progname_full` 已从 `pn` 初始化
- `__hwcap` 已从 `AT_HWCAP` 设置
- `LIBC.need_locks` 已根据是否有线程能力设置

---

### __init_tls

```rust
// Rust — 线程局部存储 (TLS) 初始化
fn __init_tls(auxv: *mut usize);
```

[Visibility]: Internal (不导出) — TLS (线程局部存储) 初始化

**前置条件**:
1. 在 `__init_libc` 之后调用
2. `auxv` 指向 auxiliary vector

**后置条件**:
- 主线程的 TLS 块已分配并初始化
- `LIBC.tls_head` 链表已构建
- `LIBC.tls_size`, `LIBC.tls_align`, `LIBC.tls_cnt` 已计算
- 若架构使用 TLS 寄存器(如 x86 的 `%fs`)，该寄存器已设置

---

### __init_ssp

```rust
// Rust — Stack Smashing Protector 初始化
fn __init_ssp(entropy: *mut c_void);
```

[Visibility]: Internal (不导出) — Stack Smashing Protector 初始化

**前置条件**:
1. 在 `__init_libc` 之后调用
2. `entropy` 指向至少 `size_of::<usize>()` 字节的随机数据

**后置条件**:
- `__stack_chk_guard` 已设置为基于 `entropy` 的 canary 值

---

### __libc_start_init

```rust
// Rust — 调用所有共享库和主程序的初始化函数
fn __libc_start_init();
```

[Visibility]: Internal (不导出) — 调用所有共享库和主程序的初始化函数

**前置条件**: TLS 已初始化，libc 状态已就绪

**后置条件**:
- 所有共享库的 `.init` / `.init_array` 段中的初始化函数已被调用
- 主程序的 `.init_array` 函数已被调用
- 调用顺序: 先依赖库，后主程序

---

### __funcs_on_exit

```rust
// Rust — 执行 atexit 注册的所有退出处理函数
fn __funcs_on_exit();
```

[Visibility]: Internal (不导出) — 执行 `atexit` 注册的所有退出处理函数

**前置条件**: 由 `exit()` 或 `return from main` 调用

**后置条件**:
- 所有通过 `atexit()` 注册的函数按注册的**逆序**被调用
- 所有 stdio 流被刷新(`fflush(NULL)`)

---

### __funcs_on_quick_exit

```rust
// Rust — 执行 at_quick_exit 注册的快速退出处理函数
fn __funcs_on_quick_exit();
```

[Visibility]: Internal (不导出) — 执行 `at_quick_exit` 注册的快速退出处理函数

**前置条件**: 由 `quick_exit()` 调用

**后置条件**: 所有通过 `at_quick_exit()` 注册的函数按注册的逆序被调用

---

### __libc_exit_fini

```rust
// Rust — 调用共享库的终止函数
fn __libc_exit_fini();
```

[Visibility]: Internal (不导出) — 调用共享库的终止函数

**前置条件**: 由 `exit()` 在 `__funcs_on_exit` 之后调用

**后置条件**: 所有共享库的 `.fini` / `.fini_array` 段中的终止函数被调用(逆序)

---

### __fork_handler

```rust
// Rust — fork 锁处理的中央调度函数
fn __fork_handler(who: c_int);
```

[Visibility]: Internal (不导出) — fork 锁处理的中央调度函数

**前置条件**:
- `who` in {`-1`, `0`, `1`}: prepare / parent / child
- 被 `fork()` 实现调用

**后置条件**: `__fork_handler` 是 fork 锁管理的中央调度器，按正确顺序调用 `__malloc_atfork`, `__ldso_atfork`, `__pthread_key_atfork`，以及 `__post_Fork`。

**Intent**: `__fork_handler` 是 fork 锁管理的中央调度器，按正确顺序调用所有 atfork 回调。

---

### __synccall

```rust
// Rust — 在所有线程上同步执行指定函数
fn __synccall(func: extern "C" fn(*mut c_void), ctx: *mut c_void);
```

[Visibility]: Internal (不导出) — 在所有线程上同步执行指定函数

**前置条件**:
1. 进程是多线程的(`LIBC.threads_minus_1 >= 1`)，或者仅有主线程(此时直接调用)
2. `func` 不可为 `null`
3. `ctx` 为传递给 `func` 的不透明上下文指针

**后置条件**:
- `func(ctx)` 已在**每个活跃线程**的上下文中被调用
- 调用期间所有其他线程被暂停(通过信号 `SIGSYNCCALL` 实现)
- 调用者线程也执行 `func(ctx)`

**System Algorithm**: 使用 `SIGSYNCCALL` 信号广播给所有线程，每个线程在信号处理器中执行 `func(ctx)` 并等待所有线程完成(通过屏障同步)，然后统一返回。

**Intent**: 实现全局同步操作。例如 `setuid()` 在多线程程序中需要通过 `__synccall` 确保每个线程的 UID 都被设置。

---

### __setxid

```rust
// Rust — 在多线程程序中同步设置 UID/GID
fn __setxid(nr: c_int, id: c_int, eid: c_int, sid: c_int) -> c_int;
```

[Visibility]: Internal (不导出) — 在多线程程序中同步设置 UID/GID

**前置条件**:
1. `nr` 为系统调用号(`SYS_setuid`, `SYS_setgid`, `SYS_setreuid`, `SYS_setregid`, `SYS_setresuid`, `SYS_setresgid` 之一)
2. `id`, `eid`, `sid` 的含义取决于 `nr` 指定的系统调用

**后置条件**:
- 调用线程的凭据已通过系统调用设置
- 若进程是多线程的，通过 `__synccall` 同步到所有线程
- 返回 0 成功，或 `-1` 且设置 `errno`

**Intent**: POSIX 要求 `setuid()` 等函数在多线程程序中设置**所有线程**的 UID，而不仅仅是调用线程。`__setxid` 通过 `__synccall` 确保同步。

---

## 实现指南 (rusl/Rust)

- `Libc` 使用 `#[repr(C)]` 结构体，确保与 TLS 和汇编代码的布局一致
- 全局状态 `LIBC` 使用 `static mut` 声明；在单线程初始化阶段访问无竞争；多线程阶段访问时需配合锁
- `need_locks` 字段需通过 `core::ptr::read_volatile` 读取，确保每次访问从内存重载(对应 C 的 `volatile`)
- `program_invocation_short_name` 和 `program_invocation_name` 使用 `#[no_mangle] pub static mut` 导出为 C ABI 兼容的全局符号
- 所有初始化函数内部的 unsafe 块应仅限于 FFI 调用和裸指针操作，主体逻辑尽量使用安全 Rust
- 考虑使用 `core::sync::atomic::AtomicI32` 替代 `volatile signed char` 实现 `need_locks` 的更精确内存顺序