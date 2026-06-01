# __libc_start_main.c 规约

> musl libc C 运行时入口点实现，负责从 ELF 入口 `_start` 到用户 `main()` 之间的所有初始化与过渡工作。

---

## 依赖图

```
_start (crt1.o)
  └─> __libc_start_main
        ├─> __init_libc
        │     ├─> (设置 __environ, libc.auxv, __hwcap, __sysinfo, libc.page_size)
        │     ├─> (提取 __progname / __progname_full)
        │     ├─> __init_tls  (see __init_tls.c spec)
        │     ├─> __init_ssp  (栈保护初始化，默认为 dummy1)
        │     └─> [SUID/SGID 检查] → __syscall(SYS_poll/SYS_ppoll) / __sys_open / a_crash
        └─> libc_start_main_stage2
              ├─> __libc_start_init (→ libc_start_init)
              │     ├─> _init()  (可由用户重定义，默认 dummy)
              │     └─> 遍历 __init_array 段调用构造器
              └─> exit(main(argc, argv, envp))
```

---

## 全局变量与状态结构

### Struct __libc (定义于 src/internal/libc.h)

```c
struct __libc {
    char can_do_threads;        // 是否支持线程（由 __init_tp 设置）
    char threaded;              // 是否处于多线程模式
    char secure;                // 是否处于安全执行模式（SUID/SGID）
    volatile signed char need_locks;  // 是否需要锁
    int threads_minus_1;        // 线程数减一
    size_t *auxv;               // 辅助向量指针
    struct tls_module *tls_head; // TLS 模块链表头
    size_t tls_size, tls_align, tls_cnt;  // TLS 布局参数
    size_t page_size;           // 系统页大小
    struct __locale_struct global_locale; // 全局 locale
};
```

[Visibility]: Internal — musl 内部全局状态结构，POSIX 标准未定义

### 全局变量 (均定义于 src/env/ 及 libc.h 声明)

| 变量 | 类型 | 含义 | Visibility |
|------|------|------|------------|
| `libc` / `__libc` | `struct __libc` | libc 全局状态 | Internal |
| `__environ` | `char **` | 环境变量指针数组 | Internal (通过 `environ` weak_alias 暴露为 POSIX) |
| `__progname` | `char *` | 程序名（不含路径） | Internal |
| `__progname_full` | `char *` | 程序完整路径名 | Internal |
| `__hwcap` | `size_t` | CPU 硬件能力位掩码 | Internal |
| `__sysinfo` | `size_t` | Linux vDSO sysinfo 地址 | Internal |

---

## 常量定义

### AUX_CNT

```c
#define AUX_CNT 38
```

[Visibility]: Internal — 文件内部宏

- **Intention**: 定义辅助向量本地缓存数组的长度。该值必须大于等于内核可能传递的所有 `AT_*` 类型的最大索引值，当前 musl 使用的最大索引为 `AT_SYSINFO_EHDR`（33）。

### AT_* 常量（定义于 `<elf.h>`）

- `AT_HWCAP` (16) — CPU 硬件能力位掩码
- `AT_SYSINFO` (32) — vDSO `__kernel_vsyscall` 入口地址
- `AT_PAGESZ` (6) — 系统页大小
- `AT_EXECFN` (31) — 可执行文件路径
- `AT_RANDOM` (25) — 16 字节随机数（用于栈保护 canary）
- `AT_UID` (11), `AT_EUID` (12), `AT_GID` (13), `AT_EGID` (14) — 实际/有效用户和组 ID
- `AT_SECURE` (23) — 非零表示需要安全模式（如 SUID 二进制）

---

## 函数规约

### 1. dummy (内部静态函数)

```c
static void dummy(void) {}
```

[Visibility]: Internal — musl 内部占位函数，不对外导出

- **Intention**: 提供 `_init` 的默认空实现。若用户未定义 `_init()` 函数，链接器将使用此弱符号默认值。

**前置条件**: 无。

**后置条件**: 无操作，立即返回。


### 2. dummy1 (内部静态函数)

```c
static void dummy1(void *p) {}
```

[Visibility]: Internal — musl 内部占位函数，不对外导出

- **Intention**: 提供 `__init_ssp`（栈保护初始化）的默认空实现。若未启用栈保护，则该函数为空操作。

**前置条件**: 无。

**后置条件**: 无操作，忽略参数 `p`，立即返回。


### 3. \_init (weak_alias)

```c
weak_alias(dummy, _init);
```

[Visibility]: Internal (间接 Public) — 默认实现为 `dummy()`，但用户可在程序中定义 `void _init(void)` 来覆盖该弱符号，从而在 `main()` 之前执行自定义初始化。

- **Intention**: 提供与 legacy System V ABI 兼容的初始化钩子。若用户未定义 `_init`，则不做任何事。

**前置条件**:
- 若用户定义了 `_init()`：调用前栈和 TLS 已就绪，但可能尚未调用 `.init_array` 中的构造器
- `libc_start_init()` 会首先调用 `_init()`，之后再遍历 `.init_array`

**后置条件**: 返回后，`.init_array` 中的构造器将被依次调用。

**不变量**: `_init()` 在 `__libc_start_main` 的整个生命周期中 **恰好被调用一次**（通过 `libc_start_init`）。


### 4. \_\_init_ssp (weak_alias)

```c
weak_alias(dummy1, __init_ssp);
```

[Visibility]: Internal — musl 内部栈保护初始化入口

- **Intention**: 栈粉碎保护器（Stack Smashing Protector, SSP）的初始化入口。默认实现为空函数；若编译时启用 `-fstack-protector`，实际实现会从内核提供的 `AT_RANDOM` 数据中设置栈 canary。

**前置条件**:
- 调用时参数为 `aux[AT_RANDOM]` 的值，即指向 16 字节内核随机数的指针
- `__init_tls` 已经完成（在 `__init_libc` 中先调用 `__init_tls` 后调用 `__init_ssp`）

**后置条件**: 若 `AT_RANDOM` 非空，栈 canary 已被设置。


### 5. \_\_init_libc (hidden, 外部可见)

```c
void __init_libc(char **envp, char *pn)
```

函数签名见 `src/internal/libc.h` 第 40 行。

[Visibility]: Internal — musl 内部 libc 初始化函数，由 `__libc_start_main` 和动态链接器调用，不对外部用户暴露

#### Intent

本函数是整个 libc 的初始化中枢，负责从内核传递的辅助向量中提取系统参数、初始化全局 libc 状态、设置 TLS 和栈保护，并检测 SUID/SGID 安全执行模式。

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
- `libc.auxv` 指向内核传递的辅助向量
- `__hwcap` 被设置为 `AT_HWCAP` 对应的 CPU 硬件能力掩码
- `__sysinfo` 被设置为 `AT_SYSINFO` 对应的 vDSO 入口地址（若存在）
- `libc.page_size` 被设置为系统页大小（通过 `AT_PAGESZ`）
- `__progname_full` 被设置为程序路径名，`__progname` 被设置为纯文件名部分（不含路径分隔符）
- TLS 初始化已完成（通过 `__init_tls(aux)`）
- 栈保护 canary 已设置（通过 `__init_ssp((void*)aux[AT_RANDOM])`）
- `libc.secure` 保持为 0

**Case 2: SUID/SGID 安全执行模式**（`AT_UID == AT_EUID && AT_GID == AT_EGID && AT_SECURE != 0`）

- 除 Case 1 的所有效果外，额外执行安全加固：
  - 对文件描述符 0、1、2 执行 `poll(..., 0)` 检查
  - 若任何标准文件描述符不存在（`POLLNVAL`），将其重定向到 `/dev/null`
  - 若 `poll` 系统调用失败，调用 `a_crash()` 终止进程
  - `libc.secure` 被设置为 1

#### 系统算法

```
__init_libc(envp, pn):
  1. 设置 __environ = envp
  2. 计算环境变量数量 i，定位 auxv = envp + i + 1
  3. 解析 auxv 到本地数组 aux[AUX_CNT]：
     for each {type, value} pair in auxv:
       if type < AUX_CNT: aux[type] = value
  4. 提取全局系统参数：
     __hwcap     = aux[AT_HWCAP]
     __sysinfo   = aux[AT_SYSINFO] (if non-zero)
     page_size   = aux[AT_PAGESZ]
  5. 设置程序名：
     if pn == NULL: 尝试 aux[AT_EXECFN]
     __progname_full = pn
     __progname = pn 的纯文件名部分 (最后一个 '/' 之后)
  6. 初始化 TLS:  __init_tls(aux)
  7. 初始化 SSP:  __init_ssp((void*)aux[AT_RANDOM])
  8. 安全模式检测:
     if AT_UID != AT_EUID || AT_GID != AT_EGID || AT_SECURE != 0:
       - 对 fd 0,1,2 调用 poll(..., 0)
       - 若有 fd 无效，打开 /dev/null 并 dup 到无效 fd
       - 若任何系统调用失败，调用 a_crash()
       - 设置 libc.secure = 1
     else:
       - 直接返回（不设置 libc.secure）
```

#### 不变量

- `libc.auxv` 在整个进程生命周期中始终指向同一块内存（一次设置，永不更改）
- `libc.page_size` 在整个进程生命周期中保持不变
- `__environ` 的值与主函数 `main(argc, argv, envp)` 中的 `envp` 参数一致
- 函数返回后，文件描述符 0、1、2 保证有效（指向终端、管道或 `/dev/null`）

#### 依赖

- `__init_tls(size_t *aux)` — 定义于 `src/env/__init_tls.c`（see `__init_tls.c` spec）
- `__init_ssp(void *p)` — 默认 `dummy1`，可能由编译器生成的栈保护代码重定义
- `__syscall(SYS_poll, ...)` / `__syscall(SYS_ppoll, ...)` — 系统调用
- `__sys_open(...)` — 系统调用宏
- `a_crash()` — 定义于 `src/internal/atomic.h`，通过写入空指针触发 SIGSEGV


### 6. libc_start_init (内部静态函数)

```c
static void libc_start_init(void)
```

[Visibility]: Internal — musl 内部用户代码初始化函数，通过 `__libc_start_init` 弱别名暴露

#### Intent

调用用户定义的初始化函数：首先调用传统的 `_init()` 函数（若用户提供），然后依次调用 `.init_array` 段中由编译器和链接器放置的所有构造函数（如 C++ 静态对象构造器、`__attribute__((constructor))` 函数等）。

#### 前置条件

- `__init_libc` 已成功返回（TLS、栈保护等基础设施就绪）
- 单线程执行环境
- `__init_array_start` 和 `__init_array_end` 由链接器定义，标记 `.init_array` 段的起止地址
- 若 `.init_array` 段为空，两者地址相等

#### 后置条件

- `_init()` 函数已被调用（若用户定义了则执行用户代码，否则执行 `dummy()` 空操作）
- `.init_array` 段中的所有构造器函数指针已按地址升序逐一调用
- 所有构造器均已返回（未捕获的异常或 `longjmp` 视为异常情况，不在本规约范围内）

#### 系统算法

```
libc_start_init():
  _init()                                // Step 1: legacy init
  a = &__init_array_start
  while a < &__init_array_end:           // Step 2: .init_array
    call function pointer at address a
    a += sizeof(void(*)())
```

#### 不变量

- `.init_array` 中的函数指针按存储顺序调用（地址升序）
- 每个构造器恰好被调用一次


### 7. \_\_libc_start_init (weak_alias)

```c
weak_alias(libc_start_init, __libc_start_init);
```

[Visibility]: Internal — musl 内部符号，供 `__libc_start_main` 调用

- **Intention**: 将静态函数 `libc_start_init` 通过弱别名暴露为 `__libc_start_init`，使 `__libc_start_main` 阶段二的代码能够访问。

前置/后置条件及行为：完全等同于 `libc_start_init`。


### 8. libc_start_main_stage2 (内部静态函数)

```c
static int libc_start_main_stage2(int (*main)(int, char **, char **), int argc, char **argv)
```

[Visibility]: Internal — musl 内部第二阶段启动函数，不对外导出

#### Intent

启动流程的第二阶段（也是最后一阶段）。与第一阶段 `__libc_start_main` 分离为一个独立函数，目的是通过"函数返回"而非"函数调用"来释放第一阶段使用的栈帧，避免启动代码的栈帧在进程整个生命周期中持续占用空间。此函数负责调用用户初始化函数并将控制权移交给用户 `main()`。

#### 前置条件

- `__init_libc` 已在第一阶段成功返回
- `argv` 指向有效的参数数组，`envp = argv + argc + 1`
- 栈帧隔离屏障（`__asm__` 约束）已完成

#### 后置条件

**本函数不返回**（通过 `exit()` 终止）:

1. 调用 `__libc_start_init()` 执行用户初始化
2. 调用 `main(argc, argv, envp)` 执行用户程序
3. 将 `main()` 的返回值传递给 `exit()`
4. `return 0` 语句不可达（仅为满足编译器要求的 `-Wreturn-type`）

#### 系统算法

```
libc_start_main_stage2(main, argc, argv):
  envp = argv + argc + 1
  __libc_start_init()                    // 执行用户构造器
  exit(main(argc, argv, envp))           // 调用用户 main，exit 永不返回
  return 0                               // 不可达
```

#### 不变量

- `main()` 在 `__libc_start_init()` 之后、任何 `atexit` 处理函数之前被调用
- 若进程正常终止，`exit()` 确保所有 `atexit` 注册函数和 C++ 析构函数被调用


### 9. \_\_libc_start_main (hidden, 外部可见)

```c
int __libc_start_main(
    int (*main)(int, char **, char **),
    int argc,
    char **argv,
    void (*init_dummy)(),
    void (*fini_dummy)(),
    void (*ldso_dummy)()
)
```

[Visibility]: Internal — musl CRT 运行时入口点。由 `_start`（crt1.o / Scrt1.o）调用，不是 POSIX/C 标准函数，不对应用程序开发者暴露。

#### Intent

C 语言运行时入口点，是汇编级别 `_start` 和用户 `main()` 之间的桥梁。它在单线程环境下完成所有 libc 基础设施初始化，然后将控制权安全地传递给用户程序。

#### 前置条件

- **调用者**: 仅由 CRT 启动代码 `_start` 或动态链接器的 `_dlstart` 调用
- **参数有效性**:
  - `main`: 指向用户 `main()` 函数的指针，不可为 NULL
  - `argc`: 命令行参数数量（非负整数，至少为 0）
  - `argv`: 命令行参数数组，`argv[0]` 至 `argv[argc-1]` 有效，`argv[argc] == NULL`
  - `envp`（隐式）: 环境变量数组紧随 `argv` 之后：`envp = argv + argc + 1`
  - `init_dummy`, `fini_dummy`, `ldso_dummy`: 保留参数，musl 当前忽略
- **栈状态**: 栈已被内核正确设置，`_start` 已将所有栈参数（argc, argv, envp, auxv）准备好
- **执行环境**:
  - 寄存器状态: `_start` 已完成必要的寄存器清零（如 x86_64 上 `xor %ebp, %ebp`）
  - 单线程执行，尚未启用信号处理

#### 后置条件

**本函数不直接返回**——它将控制权传递给 `libc_start_main_stage2`（进而传递给 `main()`），最终进程通过 `exit()` 终止。

**状态转换**（从调用到最终完成）:

1. **libc 初始化状态**（由 `__init_libc` 保证）:
   - `__environ`、`__progname`、`__progname_full` 已设置
   - `libc.page_size`、`libc.auxv`、`__hwcap` 已设置
   - `__sysinfo` 已设置（若 vDSO 可用）
   - TLS 已初始化，线程指针已设置
   - 若 SUID/SGID 程序：`libc.secure = 1`，fd 0/1/2 已验证/修复

2. **编译器屏障**: 通过内联汇编 `__asm__("" : "+r"(stage2) : : "memory")` 阻止编译器将应用代码或 SSP/线程指针访问提升到 `__init_libc` 之前。

3. **执行流程**（由 `libc_start_main_stage2` 保证）:
   - 用户 `_init()` 和 `.init_array` 构造器被执行
   - 用户 `main(argc, argv, envp)` 被执行
   - `main()` 返回值传递给 `exit()`
   - 若 `main()` 返回：`exit(n)` 调用 `atexit` 注册函数、刷新 stdio 缓冲区、最终调用 `_exit(n)`
   - 进程以 `main()` 返回值作为退出码终止

#### 系统算法

```
__libc_start_main(main, argc, argv, init_dummy, fini_dummy, ldso_dummy):
  // 第一阶段：libc 初始化
  envp = argv + argc + 1

  // __init_libc 被声明为 __noinline__ 以限制其栈帧作用域
  __init_libc(envp, argv[0])

  // 编译器屏障：防止应用代码、
  // SSP 访问、线程指针访问在 TLS/SSP 初始化之前被提升
  stage2 = libc_start_main_stage2
  __asm__ volatile("" : "+r"(stage2) : : "memory")

  // 第二阶段：通过函数调用"返回"到 stage2
  // 这使得 stage2 获得新的栈帧，释放第一阶段的栈空间
  return stage2(main, argc, argv)
```

**两阶段设计原理**:

- 第一阶段 `__init_libc` 的栈帧在 `__libc_start_main` 中创建。通过将第二阶段实现为独立函数 `libc_start_main_stage2`，并让 `__libc_start_main` 调用它，编译器在 `libc_start_main_stage2` 中创建新的栈帧。当 `__libc_start_main` 返回时，其栈帧被释放。
- 编译器屏障 (`__asm__`) 确保编译器不会将二阶段代码优化提升到屏障之前，防止在 TLS 和栈保护初始化完成之前访问线程指针/SSP。

#### 不变量

- 整个初始化序列在单线程环境下执行（`libc.threads_minus_1 == 0`）
- `__libc_start_main` **恰好被调用一次**
- `main()` **恰好被调用一次**
- 若 `main()` 返回（而非调用 `exit()` 本身），`exit()` 处理程序终止

#### 依赖

- `__init_libc(char **envp, char *pn)` — 本文件定义
- `libc_start_main_stage2(main, argc, argv)` — 本文件定义（静态）
- `exit(int status)` — 外部依赖（stdlib），由 `libc_start_main_stage2` 调用

---

## 外部弱符号（链接器定义）

### \_\_init_array_start / \_\_init_array_end

```c
extern weak hidden void (*const __init_array_start)(void), (*const __init_array_end)(void);
```

[Visibility]: Internal — 由链接器根据 `.init_array` 段自动定义，非 C 代码定义

- **来源**: 由 GNU ld / lld 链接器在链接 `.init_array` 段时自动生成
- **含义**: 标记 `.init_array` 段的起止地址，该段存储构造函数指针数组
- **约束**: 若 `.init_array` 段为空，两者值相等（或为 NULL）；若非空，`__init_array_start < __init_array_end`

---

## 关键设计约束

1. **单线程假设**: 所有初始化均在主线程创建前完成，无需同步。
2. **不可分配内存**: `__init_libc` 阶段不应调用 `malloc`，因为此时 allocator 可能未初始化。
3. **有限系统调用**: 仅使用内核保证可用的基础系统调用（`poll`/`ppoll`、`open`）。
4. **安全模式**: SUID/SGID 程序需要在用户代码获得控制权之前完成 fd 验证和环境清理。
5. **栈帧隔离**: 两阶段设计确保初始化栈帧不会污染主程序栈空间（对深度递归的嵌入式程序至关重要）。