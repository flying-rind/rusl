# __libc_start_main 规约 (Rust)

> rusl C 运行时入口点实现，负责从 ELF 入口 `_start` 到用户 `main()` 之间的所有初始化与过渡工作。
> `#![no_std]` 兼容；消除 C weak_alias 间接性，改用函数指针间接层。

---

## 依赖图

```
_start (crt1.S)
  └─> __libc_start_main                              // extern "C" 入口, #[no_mangle]
        ├─> __init_libc                               // extern "C", 供动态链接器复用
        │     ├─> (设置 __environ, libc.auxv, __hwcap, __sysinfo, libc.page_size)
        │     ├─> (提取 __progname / __progname_full)
        │     ├─> __init_tls                          // 外部依赖 (see __init_tls spec)
        │     ├─> __init_ssp                          // 函数指针间接层, 默认 no-op
        │     └─> [SUID/SGID 检查] → syscall(SYS_poll/SYS_ppoll) / sys_open / a_crash
        └─> libc_start_main_stage2                    // 模块私有函数 (内部 Rust ABI)
              ├─> libc_start_init                     // 模块私有函数
              │     ├─> _init()                       // 函数指针间接层, 默认 no-op
              │     └─> 遍历 __init_array 段调用构造器
              └─> exit(main(argc, argv, envp))        // 外部依赖
```

> **设计变化**: Rust 版使用函数指针间接层替代 C 的 `weak_alias` 机制。`_init`、`__init_ssp`、`__libc_start_init`
> 在 C 中通过 weak_alias 暴露为可覆盖符号；Rust 版将 `_init` 和 `__init_ssp` 建模为全局函数指针，
> `__libc_start_init` 的别名间接性则直接消除（模块内直接调用 `libc_start_init`）。

---

## 常量定义

### AUX_CNT

```rust
const AUX_CNT: usize = 38;
```

[Visibility]: Internal — 模块私有常量

- **Intention**: 定义辅助向量本地缓存数组的长度。该值必须大于等于内核可能传递的所有 `AT_*` 类型的最大索引值。当前 musl 使用的最大索引为 `AT_SYSINFO_EHDR`（33）。

### AT_* 常量（定义于 sys/elf.rs 或等效模块）

```rust
pub(crate) const AT_HWCAP:  usize = 16;  // CPU 硬件能力位掩码
pub(crate) const AT_SYSINFO: usize = 32; // vDSO __kernel_vsyscall 入口地址
pub(crate) const AT_PAGESZ:  usize = 6;  // 系统页大小
pub(crate) const AT_EXECFN:  usize = 31; // 可执行文件路径
pub(crate) const AT_RANDOM:  usize = 25; // 16 字节随机数（用于栈保护 canary）
pub(crate) const AT_UID:     usize = 11; // 实际用户 ID
pub(crate) const AT_EUID:    usize = 12; // 有效用户 ID
pub(crate) const AT_GID:     usize = 13; // 实际组 ID
pub(crate) const AT_EGID:    usize = 14; // 有效组 ID
pub(crate) const AT_SECURE:  usize = 23; // 非零表示需要安全模式
```

[Visibility]: Internal — rusl 内部常量

---

## 全局状态

### 结构体 LibcState（定义于 src/internal/libc.rs）

```rust
/// libc 全局状态结构体（零初始化兼容）
///
/// 该结构体作为 BSS 段静态变量存在，保证零初始化语义，
/// 使 _start 调用 __libc_start_main 前所有字段已归零。
#[repr(C)]
pub(crate) struct LibcState {
    pub(crate) can_do_threads:   u8,            // char -> u8, 是否支持线程
    pub(crate) threaded:         u8,            // char -> u8, 是否多线程模式
    pub(crate) secure:           u8,            // char -> u8, 安全执行模式
    pub(crate) need_locks:       core::sync::atomic::AtomicI8, // volatile signed char -> AtomicI8
    pub(crate) threads_minus_1:  i32,           // 线程数减一 (c_int)
    pub(crate) auxv:             *mut usize,    // 辅助向量指针 (裸指针)
    pub(crate) tls_head:         *mut TlsModule,// TLS 模块链表头
    pub(crate) tls_size:         usize,
    pub(crate) tls_align:        usize,
    pub(crate) tls_cnt:          usize,         // TLS 布局参数
    pub(crate) page_size:        usize,         // 系统页大小
    pub(crate) global_locale:    LocaleStruct,  // 全局 locale
}
```

[Visibility]: Internal — rusl 内部全局状态结构，POSIX 标准未定义

### 初始化钩子表（替代 C weak_alias）

```rust
/// 初始化钩子表：替代 C 的 weak_alias 间接性
///
/// 在 __libc_start_main 调用前，以下函数指针均初始化为各自的默认实现。
/// 外部模块（如栈保护模块）可在链接阶段通过静态初始化或模块间的
/// 初始化顺序约定替换这些指针，达到与 C weak_alias 等价的效果。
pub(crate) struct InitHooks {
    /// 对应 C: weak_alias(dummy, _init)
    /// 默认值: dummy
    pub(crate) init_fn:     unsafe extern "C" fn(),
    /// 对应 C: weak_alias(dummy1, __init_ssp)
    /// 默认值: dummy1
    pub(crate) init_ssp_fn: unsafe extern "C" fn(*const c_void),
}

/// 初始化钩子表全局实例（零初始化 → 需在 __init_libc 之前显式设置默认值）
pub(crate) static mut INIT_HOOKS: InitHooks = InitHooks {
    init_fn: dummy,
    init_ssp_fn: dummy1,
};
```

[Visibility]: Internal — rusl 内部抽象

### 内部全局变量 (定义于 src/env/ 及 libc.rs 声明)

| 变量 | Rust 类型 | 含义 | Visibility |
|------|-----------|------|------------|
| `LIBC` | `UnsafeCell<LibcState>` (BSS 静态) | libc 全局状态 | Internal |
| `__environ` | `UnsafeCell<*mut *mut c_char>` | 环境变量指针数组 | Internal（通过 `environ` weak 别名暴露为 POSIX） |
| `__progname` | `UnsafeCell<*const c_char>` | 程序名（不含路径） | Internal |
| `__progname_full` | `UnsafeCell<*const c_char>` | 程序完整路径名 | Internal |
| `__hwcap` | `UnsafeCell<usize>` | CPU 硬件能力位掩码 | Internal |
| `__sysinfo` | `UnsafeCell<usize>` | Linux vDSO sysinfo 地址 | Internal |

[Visibility]: Internal — 所有全局变量均为 `pub(crate)` 或更小可见性

> **设计说明**: Rust 不直接支持 C 的 BSS 段零初始化语义。以上全局状态通过 `static` 配合
> `UnsafeCell` 实现与 C 等价的零初始化行为。因初始化阶段为单线程，这些 `UnsafeCell`
> 在 `__init_libc` 调用期间无需同步开销；多线程启动后由 `need_locks`（`AtomicI8`）控制访问协议。

---

## 函数规约

### 1. dummy (内部函数)

```rust
unsafe extern "C" fn dummy()
```

[Visibility]: Internal — rusl 内部占位函数，不对外导出

- **Intention**: 提供 `_init` 钩子的默认空实现。当无用户代码覆盖 `INIT_HOOKS.init_fn` 时使用。

**前置条件**: 无。

**后置条件**: 无操作，立即返回。

**规约等价性**: 对应 C 的 `static void dummy(void) {}`。使用 `unsafe extern "C"` 函数指针类型以匹配 `InitHooks` 的函数签名要求。


### 2. dummy1 (内部函数)

```rust
unsafe extern "C" fn dummy1(_p: *const c_void)
```

[Visibility]: Internal — rusl 内部占位函数，不对外导出

- **Intention**: 提供 `__init_ssp` 钩子的默认空实现。

**前置条件**: 无。

**后置条件**: 无操作，忽略参数，立即返回。

**规约等价性**: 对应 C 的 `static void dummy1(void *p) {}`。


### 3. \_init (可选弱符号桩)

```rust
#[no_mangle]
pub unsafe extern "C" fn _init()
```

[Visibility]: Internal（间接 Public）— 默认实现委托给 `INIT_HOOKS.init_fn`（即 `dummy`）。
`#[no_mangle]` 保留外部链接能力，允许用户在链接时提供自己的 `_init` 实现覆盖默认行为。

> **设计说明**: 仅 `_init` 保留 `#[no_mangle]` 外部符号以维持 System V ABI 兼容性。
> `__init_ssp` 的覆盖通过修改 `INIT_HOOKS.init_ssp_fn` 函数指针实现。

**前置条件**:
- 若用户链接了自定义 `_init()`：调用前栈和 TLS 已就绪
- `libc_start_init()` 会首先调用 `_init()`（通过 `INIT_HOOKS.init_fn`），之后再遍历 `.init_array`

**后置条件**: 返回后，`.init_array` 中的构造器将被依次调用。

**不变量**: `_init()`（或等价的 `INIT_HOOKS.init_fn`）在 `__libc_start_main` 的整个生命周期中恰好被调用一次。


### 4. \_\_init_ssp (函数指针间接层)

```rust
// 实际调用点为: INIT_HOOKS.init_ssp_fn(p)
// 默认行为: dummy1(p), 即空操作
```

[Visibility]: Internal — rusl 内部栈保护初始化入口，通过 `INIT_HOOKS.init_ssp_fn` 函数指针调用

- **Intention**: 栈粉碎保护器（Stack Smashing Protector, SSP）的初始化入口。默认实现为 `dummy1`
  空函数；若编译时启用 `-fstack-protector`，SSP 模块在初始化早期将 `init_ssp_fn` 替换为
  实际实现（从内核提供的 `AT_RANDOM` 数据中设置栈 canary）。

**前置条件**:
- 调用时参数为 `aux[AT_RANDOM]` 的值，即指向 16 字节内核随机数的指针
- `__init_tls` 已经完成（在 `__init_libc` 中先调用 `__init_tls` 后调用 `__init_ssp`）

**后置条件**: 若 `AT_RANDOM` 非空，栈 canary 已被设置。

**规约等价性**: 对应 C 的 `weak_alias(dummy1, __init_ssp)`。因 Rust 不直接支持弱符号链接语义，
改用编译时确定的函数指针间接层；实际效果等价。


### 5. \_\_init_libc (hidden, 外部可见)

```rust
#[no_mangle]
#[inline(never)]
unsafe extern "C" fn __init_libc(envp: *mut *mut c_char, pn: *const c_char)
```

[Visibility]: Internal — rusl 内部 libc 初始化函数，由 `__libc_start_main` 和动态链接器调用，
不对外部用户暴露。使用 `#[no_mangle]` 与 `extern "C"` 保持 ABI 兼容，以便动态链接器
（可能为非 Rust 代码）能够调用。`#[inline(never)]` 限制其栈帧作用域（保持两阶段隔离）。

#### Intent

本函数是整个 libc 的初始化中枢，负责从内核传递的辅助向量中提取系统参数、初始化全局 libc
状态、设置 TLS 和栈保护，并检测 SUID/SGID 安全执行模式。

#### 前置条件

- **调用时机**: 必须在 `_start` 之后、`main()` 之前调用，且必须在其他任何 libc 函数之前执行
- **参数有效性**:
  - `envp`: 指向环境变量字符串数组（`argv + argc + 1` 位置），不为 NULL
  - `pn`: 程序名指针（通常为 `argv[0]`），可能为 NULL（若 `argv[0]` 为 NULL）
  - 栈上内存可正常访问（内核已正确设置 `auxv` 紧跟在 `envp` 之后）
- **系统状态**:
  - 单线程执行（此时未创建任何线程）
  - 文件描述符 0/1/2 可能未打开（若父进程关闭了它们）
  - `EUID`、`EGID`、`UID`、`GID` 已由内核正确设置

#### 后置条件

**Case 1: 正常执行**（非 SUID/SGID 或辅助向量中 `AT_SECURE == 0`）

- `__environ` 被设置为 `envp`
- `LIBC` 中的 `auxv` 指向内核传递的辅助向量
- `__hwcap` 被设置为 `AT_HWCAP` 对应的 CPU 硬件能力掩码
- `__sysinfo` 被设置为 `AT_SYSINFO` 对应的 vDSO 入口地址（若存在）
- `LIBC` 中的 `page_size` 被设置为系统页大小（通过 `AT_PAGESZ`）
- `__progname_full` 被设置为程序路径名，`__progname` 被设置为纯文件名部分（不含路径分隔符）
- TLS 初始化已完成（通过 `__init_tls(aux)`）
- 栈保护 canary 已设置（通过 `INIT_HOOKS.init_ssp_fn(aux[AT_RANDOM] as *const c_void)`）
- `LIBC` 中的 `secure` 保持为 0

**Case 2: SUID/SGID 安全执行模式**（`AT_UID == AT_EUID && AT_GID == AT_EGID && AT_SECURE != 0`）

- 除 Case 1 的所有效果外，额外执行安全加固：
  - 对文件描述符 0、1、2 执行 `poll(..., 0)` 检查
  - 若任何标准文件描述符不存在（`POLLNVAL`），将其重定向到 `/dev/null`
  - 若 `poll` 系统调用失败，调用 `a_crash()` 终止进程
  - `LIBC` 中的 `secure` 被设置为 1

#### 系统算法

```
__init_libc(envp, pn):
  1. 设置 __environ = envp
  2. 计算环境变量数量 i，定位 auxv = envp + i + 1        // 裸指针运算
  3. 解析 auxv 到本地数组 aux: [usize; AUX_CNT]           // 栈上固定大小数组
     for each {type, value} pair in auxv:
       if type < AUX_CNT: aux[type] = value
  4. 提取全局系统参数：
     __hwcap     = aux[AT_HWCAP]
     __sysinfo   = aux[AT_SYSINFO] (if non-zero)
     page_size   = aux[AT_PAGESZ]
  5. 设置程序名：
     if pn == NULL: 尝试 aux[AT_EXECFN]
     __progname_full = pn
     __progname = pn 的纯文件名部分 (最后一个 '/' 之后, 从后向前扫描)
  6. 初始化 TLS:  __init_tls(&aux as *const usize)
  7. 初始化 SSP:  INIT_HOOKS.init_ssp_fn(aux[AT_RANDOM] as *const c_void)
  8. 安全模式检测:
     if AT_UID != AT_EUID || AT_GID != AT_EGID || AT_SECURE != 0:
       - 对 fd 0,1,2 调用 poll(..., 0)
       - 若有 fd 无效 (POLLNVAL), 打开 /dev/null 并 dup2 到无效 fd
       - 若任何系统调用失败, 调用 a_crash()
       - 设置 libc.secure = 1
     else:
       - 直接返回（不设置 libc.secure）
```

#### 不变量

- `LIBC.auxv` 在整个进程生命周期中始终指向同一块内存（一次设置，永不更改）
- `LIBC.page_size` 在整个进程生命周期中保持不变
- `__environ` 的值与主函数 `main(argc, argv, envp)` 中的 `envp` 参数一致
- 函数返回后，文件描述符 0、1、2 保证有效（指向终端、管道或 `/dev/null`）

#### 依赖

- `__init_tls(aux: *const usize)` — 定义于 `src/env/__init_tls.rs`（see `__init_tls` spec）
- `INIT_HOOKS.init_ssp_fn` — 全局函数指针，默认 `dummy1`
- `syscall!(SYS_poll, ...)` / `syscall!(SYS_ppoll, ...)` — 系统调用宏
- `sys_open(path, flags, mode)` — 系统调用封装
- `a_crash() -> !` — 定义于 `src/internal/atomic.rs`，通过写入空指针触发 SIGSEGV


### 6. libc_start_init (模块私有函数)

```rust
unsafe fn libc_start_init()
```

[Visibility]: Internal — rusl 内部用户代码初始化函数，模块私有

#### Intent

调用用户定义的初始化函数：首先调用 `INIT_HOOKS.init_fn`（即 `_init`），然后依次调用
`.init_array` 段中由编译器和链接器放置的所有构造函数。

#### 前置条件

- `__init_libc` 已成功返回（TLS、栈保护等基础设施就绪）
- 单线程执行环境
- `__init_array_start` 和 `__init_array_end` 由链接器定义，标记 `.init_array` 段的起止地址
- 若 `.init_array` 段为空，两者地址相等

#### 后置条件

- `INIT_HOOKS.init_fn` (即 `_init`) 已被调用
- `.init_array` 段中的所有构造器函数指针已按地址升序逐一调用
- 所有构造器均已返回

#### 系统算法

```
libc_start_init():
  INIT_HOOKS.init_fn()                                 // Step 1: legacy init
  let start = &raw const __init_array_start as *const unsafe extern "C" fn();
  let end   = &raw const __init_array_end   as *const unsafe extern "C" fn();
  let mut ptr = start;
  while ptr < end:                                     // Step 2: .init_array
    unsafe { (*ptr)(); }
    ptr = ptr.add(1);
```

#### 不变量

- `.init_array` 中的函数指针按存储顺序调用（地址升序）
- 每个构造器恰好被调用一次

**规约等价性**: 对应 C 的 `static void libc_start_init(void)`。Rust 版本不再需要
`weak_alias(libc_start_init, __libc_start_init)` 间接层，直接在模块内部使用即可。


### 7. \_\_libc_start_init (无需单独定义)

[Visibility]: N/A — Rust 版无需此 weak_alias

- **Intention**: 在 C 实现中，`weak_alias(libc_start_init, __libc_start_init)` 仅用于将静态
  函数暴露给 `libc_start_main_stage2` 调用。Rust 版通过模块内可见性直接调用
  `libc_start_init()`，无需额外的别名符号。


### 8. libc_start_main_stage2 (模块私有函数)

```rust
unsafe fn libc_start_main_stage2(
    main: unsafe extern "C" fn(c_int, *mut *mut c_char, *mut *mut c_char) -> c_int,
    argc: c_int,
    argv: *mut *mut c_char,
) -> !
```

[Visibility]: Internal — rusl 内部第二阶段启动函数，不对外导出。返回类型 `!`（never type）
强调该函数永不返回。

#### Intent

启动流程的第二阶段（也是最后一阶段）。与第一阶段 `__libc_start_main` 分离为一个独立函数，
目的是通过"函数返回"释放第一阶段使用的栈帧。此函数负责调用用户初始化函数并将控制权移交
给用户 `main()`。

#### 前置条件

- `__init_libc` 已在第一阶段成功返回
- `argv` 指向有效的参数数组，`envp = argv.add(argc as usize + 1)`
- 编译器屏障（`compiler_fence(SeqCst)`）已完成

#### 后置条件

**本函数不返回**（通过 `exit()` 终止）:

1. 调用 `libc_start_init()` 执行用户初始化
2. 调用 `main(argc, argv, envp)` 执行用户程序
3. 将 `main()` 的返回值传递给 `exit()`
4. 返回类型 `!` 保证编译器验证不可达

#### 系统算法

```
libc_start_main_stage2(main, argc, argv):
  let envp = unsafe { argv.add(argc as usize + 1) };
  libc_start_init();                                   // 执行用户构造器
  let ret = main(argc, argv, envp);                    // 调用用户 main
  exit(ret);                                            // exit 永不返回
  // 不可达: 由 ! 返回类型保证
```

#### 不变量

- `main()` 在 `libc_start_init()` 之后、任何 `atexit` 处理函数之前被调用
- 若进程正常终止，`exit()` 确保所有 `atexit` 注册函数和 C++ 析构函数被调用

#### 规约等价性

对应 C 的 `static int libc_start_main_stage2(...)`。Rust 版返回类型改进为 `!`，
消除不可达的 `return 0` 语句；其余行为完全一致。


### 9. \_\_libc_start_main (hidden, 外部可见 — 汇编入口)

```rust
#[no_mangle]
unsafe extern "C" fn __libc_start_main(
    main: unsafe extern "C" fn(c_int, *mut *mut c_char, *mut *mut c_char) -> c_int,
    argc: c_int,
    argv: *mut *mut c_char,
    init_dummy: unsafe extern "C" fn(),
    fini_dummy: unsafe extern "C" fn(),
    ldso_dummy: unsafe extern "C" fn(),
) -> c_int
```

[Visibility]: Internal — rusl CRT 运行时入口点。由 `_start`（crt1.S / Scrt1.S）调用，不是
POSIX/C 标准函数，不对应用程序开发者暴露。使用 `#[no_mangle]` 与 `extern "C"` 保持 ABI
兼容，确保汇编级 `_start` 能透明调用。

#### Intent

C 语言运行时入口点的 Rust 等价实现，是汇编级别 `_start` 和用户 `main()` 之间的桥梁。
它在单线程环境下完成所有 libc 基础设施初始化，然后将控制权安全地传递给用户程序。

#### 前置条件

- **调用者**: 仅由 CRT 启动代码 `_start` 或动态链接器的 `_dlstart` 调用
- **参数有效性**:
  - `main`: 指向用户 `main()` 函数的指针，不可为 NULL
  - `argc`: 命令行参数数量（非负整数，至少为 0）
  - `argv`: 命令行参数数组，`argv[0]` 至 `argv[argc-1]` 有效，`argv[argc] == NULL`
  - `envp`（隐式）: 环境变量数组紧随 `argv` 之后：`argv.add(argc as usize + 1)`
  - `init_dummy`, `fini_dummy`, `ldso_dummy`: 保留参数，rusl 当前忽略
- **栈状态**: 栈已被内核正确设置，`_start` 已将所有栈参数（argc, argv, envp, auxv）准备好
- **执行环境**:
  - 寄存器状态: `_start` 已完成必要的寄存器清零（如 x86_64 上 `xor %ebp, %ebp`）
  - 单线程执行，尚未启用信号处理

#### 后置条件

**本函数不直接返回** -- 它将控制权传递给 `libc_start_main_stage2`（进而传递给 `main()`），
最终进程通过 `exit()` 终止。返回值 `c_int` 仅为满足 ABI 约定；实际控制流永不从此函数返回。

**状态转换**（从调用到最终完成）:

1. **libc 初始化状态**（由 `__init_libc` 保证）:
   - `__environ`、`__progname`、`__progname_full` 已设置
   - `LIBC.page_size`、`LIBC.auxv`、`__hwcap` 已设置
   - `__sysinfo` 已设置（若 vDSO 可用）
   - TLS 已初始化，线程指针已设置
   - 若 SUID/SGID 程序：`LIBC.secure = 1`，fd 0/1/2 已验证/修复

2. **编译器屏障**: 通过 `core::sync::atomic::compiler_fence(Ordering::SeqCst)` 阻止编译器
   将应用代码或 SSP/线程指针访问提升到 `__init_libc` 之前。

3. **执行流程**（由 `libc_start_main_stage2` 保证）:
   - 用户初始化函数（`INIT_HOOKS.init_fn` 即 `_init`）和 `.init_array` 构造器被执行
   - 用户 `main(argc, argv, envp)` 被执行
   - `main()` 返回值传递给 `exit()`
   - 若 `main()` 返回：`exit(n)` 调用 `atexit` 注册函数、刷新 stdio 缓冲区、最终调用 `_exit(n)`
   - 进程以 `main()` 返回值作为退出码终止

#### 系统算法

```
unsafe extern "C" fn __libc_start_main(main, argc, argv, init_dummy, fini_dummy, ldso_dummy) -> c_int:
  // 第一阶段：libc 初始化
  let envp = unsafe { argv.add(argc as usize + 1) };

  // __init_libc 标注为 #[inline(never)], 其栈帧在调用返回后被释放
  unsafe { __init_libc(envp, argv as *const c_char) };

  // 编译器屏障：防止应用代码、SSP 访问、线程指针访问
  // 在 TLS/SSP 初始化之前被编译器提升
  core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);

  // 第二阶段：通过函数调用"返回"到 stage2
  // stage2 获得新的栈帧，释放第一阶段的栈空间
  libc_start_main_stage2(main, argc, argv)
  // 不可达: libc_start_main_stage2 返回类型为 !
```

**两阶段设计原理**（与 C 版一致）:

- 第一阶段 `__init_libc` 的栈帧在 `__libc_start_main` 中创建。通过将第二阶段实现为独立函数
  `libc_start_main_stage2`，编译器在 `libc_start_main_stage2` 中创建新的栈帧。当
  `__libc_start_main` 返回时，其栈帧被释放。
- 编译器屏障（`compiler_fence(SeqCst)`）确保编译器不会将二阶段代码优化提升到屏障之前，
  防止在 TLS 和栈保护初始化完成之前访问线程指针/SSP。
- C 版本使用 `volatile __asm__("" : "+r"(stage2) : : "memory")`，Rust 等价物为
  `compiler_fence(SeqCst)`。

#### 不变量

- 整个初始化序列在单线程环境下执行（`LIBC.threads_minus_1 == 0`）
- `__libc_start_main` 恰好被调用一次
- `main()` 恰好被调用一次
- 若 `main()` 返回（而非调用 `exit()` 本身），`exit()` 处理程序终止

#### 依赖

- `__init_libc(envp, pn)` — 本模块定义（`#[no_mangle] extern "C"`）
- `libc_start_main_stage2(main, argc, argv)` — 本模块定义（模块私有）
- `exit(status: c_int) -> !` — 外部依赖（stdlib），由 `libc_start_main_stage2` 调用
- `compiler_fence` — 来自 `core::sync::atomic`

---

## 链接器定义的符号

### \_\_init_array_start / \_\_init_array_end

```rust
// 链接器根据 .init_array 段自动定义; Rusl 通过 extern 声明引用:
extern "C" {
    #[link_name = "__init_array_start"]
    static __init_array_start: unsafe extern "C" fn();
    #[link_name = "__init_array_end"]
    static __init_array_end: unsafe extern "C" fn();
}
```

[Visibility]: Internal — 由 GNU ld / lld 链接器在链接 `.init_array` 段时自动生成，非 Rust
代码定义

- **含义**: 标记 `.init_array` 段的起止地址，该段存储构造函数指针数组
- **约束**: 若 `.init_array` 段为空，两者值相等（或为 NULL）；若非空，
  `&raw const __init_array_start < &raw const __init_array_end`

---

## 关键设计约束

1. **单线程假设**: 所有初始化均在主线程创建前完成，无需同步。
2. **不可分配内存**: `__init_libc` 阶段不应调用 allocator，因为此时 allocator 可能未初始化。
   Rust 版应避免使用 `Box`、`Vec` 等堆分配类型；栈上固定大小 `[usize; AUX_CNT]` 数组是安全的。
3. **有限系统调用**: 仅使用内核保证可用的基础系统调用（`poll`/`ppoll`、`open`）。
4. **安全模式**: SUID/SGID 程序需要在用户代码获得控制权之前完成 fd 验证和环境清理。
5. **栈帧隔离**: 两阶段设计确保初始化栈帧不会污染主程序栈空间。
6. **`#![no_std]` 约束**: 本模块不依赖 Rust 标准库。仅使用 `core` 提供的原语（`compiler_fence`、
   `Ordering`、`AtomicI8`、`c_int`、`c_char`、`c_void` 等）。
7. **unsafe 最小化**: 模块私有函数（`libc_start_init`、`libc_start_main_stage2`）内部尽量
   缩减单个 unsafe 块范围；对外 `extern "C"` 函数因全部参数为裸指针，整体标为 `unsafe`。
8. **weak_alias 替代**: Rust 不原生支持弱符号别名。设计采用函数指针间接层：
   - `_init` → `INIT_HOOKS.init_fn`（默认 `dummy`）
   - `__init_ssp` → `INIT_HOOKS.init_ssp_fn`（默认 `dummy1`）
   - `__libc_start_init` 别名完全消除，改为直接调用 `libc_start_init()`

---

## 与 C 版本的主要差异总结

| C 机制 | Rust 等价设计 | 理由 |
|--------|-------------|------|
| `weak_alias(dummy, _init)` | `INIT_HOOKS.init_fn` 函数指针 + `#[no_mangle] extern "C" fn _init()` 桩 | Rust 无弱符号；函数指针保留内部分发灵活性，`#[no_mangle]` 保留外部链接能力 |
| `weak_alias(dummy1, __init_ssp)` | `INIT_HOOKS.init_ssp_fn` 函数指针 | 纯内部使用，函数指针足够；SSP 模块可直接修改指针 |
| `weak_alias(libc_start_init, __libc_start_init)` | 直接调用 `libc_start_init()`（模块内可见） | 别名仅用于绕过 C 的 static 限制，Rust 不需要 |
| `volatile __asm__("" : "+r"(stage2) : : "memory")` | `compiler_fence(Ordering::SeqCst)` | Rust 标准编译器屏障 |
| `static struct __libc libc` (BSS 零初始化) | `static LIBC: UnsafeCell<LibcState>` | Rust 无 BSS 语义；`UnsafeCell` 提供等价零初始化 + 内部可变性 |
| `volatile signed char need_locks` | `AtomicI8` 独立全局变量 (不在 LibcState 内) | Rust 原子类型提供等价 volatile 语义并保证无数据竞争 |
| `return stage2(main, argc, argv)` (不可能返回) | `libc_start_main_stage2(...)` 返回类型 `!` | Rust never type 比 C 的不可达 return 0 更精确 |
| `char` 布尔标志 | `u8` (非 `bool`) | `#[repr(C)]` 下 C `char` 是 `u8`/`i8`，使用 `u8` 避免 niche 优化干扰布局 |

---

## [RELY]

Predefined Structures/Functions:
  // 启动入口 (汇编)
  _start (crt1.S / Scrt1.S)
    // 调用 __libc_start_main 的汇编入口

  // TLS 初始化
  __init_tls(aux: *const usize)
    // 定义于 src/env/__init_tls.rs

  // 系统调用
  syscall!(SYS_poll, fds: *mut PollFd, nfds: u64, timeout: c_int) -> c_int
  syscall!(SYS_ppoll, fds: *mut PollFd, nfds: u64, timeout: *const Timespec, sigmask: *const Sigset) -> c_int
  sys_open(path: *const c_char, flags: c_int, mode: c_int) -> c_int
  sys_close(fd: c_int) -> c_int
  sys_dup2(old: c_int, new: c_int) -> c_int

  // 崩溃处理
  a_crash() -> !
    // 定义于 src/internal/atomic.rs, 通过写入空指针触发 SIGSEGV

  // 进程终止
  exit(status: c_int) -> !
    // 定义于 src/stdlib/ 或等效模块, 由 libc_start_main_stage2 调用

  // 链接器定义的符号
  extern "C" {
      #[link_name = "__init_array_start"]
      static __init_array_start: unsafe extern "C" fn();
      #[link_name = "__init_array_end"]
      static __init_array_end: unsafe extern "C" fn();
  }

  // 核心类型与常量 (来自 core / 内部模块)
  struct PollFd { fd: c_int, events: i16, revents: i16 }  // #[repr(C)]
  c_int = i32
  c_char = i8
  c_void = core::ffi::c_void
  AT_HWCAP, AT_SYSINFO, AT_PAGESZ, AT_EXECFN, AT_RANDOM,
    AT_UID, AT_EUID, AT_GID, AT_EGID, AT_SECURE
    // 来自 sys/elf.rs 或等效常量模块

  // 内部状态结构体
  LibcState, TlsModule, LocaleStruct
    // 定义于 src/internal/libc.rs

  // Compiler fence
  core::sync::atomic::compiler_fence
  core::sync::atomic::Ordering

[GUARANTEE]
Exported Interface:
  // CRT 入口点 (由 _start 汇编调用, 必须保持 extern "C" ABI)
  #[no_mangle]
  unsafe extern "C" fn __libc_start_main(
      main: unsafe extern "C" fn(c_int, *mut *mut c_char, *mut *mut c_char) -> c_int,
      argc: c_int,
      argv: *mut *mut c_char,
      init_dummy: unsafe extern "C" fn(),
      fini_dummy: unsafe extern "C" fn(),
      ldso_dummy: unsafe extern "C" fn(),
  ) -> c_int;

  // libc 初始化 (由动态链接器复用, 必须保持 extern "C" ABI)
  #[no_mangle]
  #[inline(never)]
  unsafe extern "C" fn __init_libc(
      envp: *mut *mut c_char,
      pn: *const c_char,
  );

  // _init 弱符号桩 (保留 #[no_mangle] 以支持 System V ABI 兼容)
  #[no_mangle]
  pub unsafe extern "C" fn _init();
    // 默认委托给 INIT_HOOKS.init_fn (即 dummy)
    // 用户可通过链接时定义同名符号覆盖
