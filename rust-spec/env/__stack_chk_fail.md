# __stack_chk_fail — Rust 接口归约

> **源 C spec**: `src/env/spec/__stack_chk_fail.md`
> **导出状态**: 本模块所有符号均为 Internal — 编译器 ABI 所需符号，非标准 C/POSIX 接口，用户程序不应直接访问。

---

## 依赖图

```
__stack_chk_guard (全局变量, #[no_mangle])
  └── __init_ssp → core::ptr::copy_nonoverlapping  (替代 C 的 memcpy, no_std 兼容)
                 → __pthread_self()                 (内部函数, pthread 模块)
                     └── .canary 字段写入

__stack_chk_fail → core::ptr::write_volatile (空指针写入, 触发 SIGSEGV)

__stack_chk_fail_local ──(弱别名/函数转发)──→ __stack_chk_fail
```

---

## 模块概述

本模块实现了 GCC/Clang 栈保护器 (Stack Smashing Protector, SSP) 的运行时支持。当程序以 `-fstack-protector` 或类似选项编译时，编译器会在每个函数的栈帧中插入一个 "canary" 值，并在函数返回前检查该值是否被篡改。若检测到栈缓冲区溢出，则调用 `__stack_chk_fail` 终止程序。

rusl 在以下环节提供 SSP 支持：
1. **Canary 初始化** (`__init_ssp`)：在程序启动早期，通过 AT_RANDOM 辅助向量提供的随机熵（或回退到确定性算法）初始化全局 canary 值 `__stack_chk_guard`。
2. **线程 Canary 同步**：将 `__stack_chk_guard` 的值复制到当前线程的 TLS 头部 `canary` 字段，确保每个线程都能正确访问 canary。
3. **栈破坏响应** (`__stack_chk_fail`)：当检测到 canary 被篡改时，通过空指针 volatile 写入触发 SIGSEGV，立即终止进程。

---

## `__stack_chk_guard` (全局变量)

```rust
#[no_mangle]
static mut __stack_chk_guard: usize = 0;
```

[Visibility]: Internal — GCC/Clang 编译器 ABI 所需的栈保护 canary 全局变量。通过 `#[no_mangle]` 导出符号名，编译器在生成栈保护代码时直接引用此符号。用户程序不应直接访问。

### 类型映射

| C 类型 | Rust 类型 | 说明 |
|--------|-----------|------|
| `uintptr_t` | `usize` | 指针宽度整数，在 32 位平台为 4 字节，64 位平台为 8 字节 |

### 前置条件
- 无。该变量在程序加载时为零初始化的 `.bss` 段中（Rust 的 `= 0` 初始值）。

### 后置条件
- 在 `__init_ssp()` 调用后，持有当前进程的栈 canary 值，该值在进程生命周期内保持不变。

### 不变量
- `__stack_chk_guard` 在 `__init_ssp()` 调用后始终非零。
- `__stack_chk_guard == __pthread_self().canary`（初始化后始终成立）。
- 在 64 位平台 (`cfg(target_pointer_width = "64")`) 上，`((&raw const __stack_chk_guard) as *const u8).add(1).read()` 返回 0：canary 的第二字节始终为零（NULL 字节），用于防御通过字符串操作函数进行的 canary 泄漏/覆盖。

### 意图
Canary 值的更新被设计为一次性初始化：`__init_ssp` 在程序启动早期调用一次，之后 `__stack_chk_guard` 仅被读取。这保证了 canary 值的不可变性，防止攻击者通过信息泄漏后覆写 canary。

---

## `__init_ssp` (内部函数)

```rust
extern "C" fn __init_ssp(entropy: *mut core::ffi::c_void);
```

[Visibility]: Internal — musl/rusl 内部栈保护初始化函数。不是 POSIX/C 标准接口，不在任何公开头文件中声明。由 rusl 的 C 运行时启动代码在程序启动早期调用。使用 `extern "C"` 保持与 C 启动代码的 ABI 兼容。

### 意图

`__init_ssp` 是栈保护机制的初始化入口。它从内核提供的辅助向量（AUXV）中获取随机熵，生成每个进程唯一、不可预测的 canary 值。当无法获取随机熵时，回退到基于地址的确定性算法（仍然提供一定程度的不可预测性，因为 ASLR 使得 `&__stack_chk_guard` 的地址随机）。

此外，在 64 位平台上刻意将 canary 第二字节清零，以防御通过 `strcpy`/`sprintf` 等字符串操作函数泄漏或覆写 canary 的攻击：攻击者如果以字符串方式溢出缓冲区，会被 NULL 字节截断而无法完成覆盖。

### 前置条件

- 调用发生在程序启动的极早期阶段，在任何用户代码之前。
- 调用发生在单线程环境中（此时仅有主线程存在）。
- `entropy` 参数由启动代码设置：
  - 非 NULL：指向从内核 `AT_RANDOM` 辅助向量获得的随机字节缓冲区（至少 `size_of::<usize>()` 字节）。
  - NULL：表示随机熵不可用（如在内核未提供 `AT_RANDOM` 的平台上）。

### 后置条件

**Case 1: 有随机熵 (`entropy` 非 NULL)**
- `__stack_chk_guard` 被设置为从 `entropy` 指向的缓冲区复制的 `size_of::<usize>()` 字节值。
- 在 64 位平台上，第二字节被强制置零，牺牲 8 位熵以换取字符串攻击防御。
- 当前线程的 `canary` 字段被设置为 `__stack_chk_guard` 的值。

**Case 2: 无随机熵 (`entropy` 为 NULL)**
- `__stack_chk_guard` 被设置为 `(&raw const __stack_chk_guard as usize).wrapping_mul(0x41C64E6D)`。
- 在 64 位平台上，第二字节同样被清零。
- 当前线程的 `canary` 字段被同步设置。
- 虽然此值是确定性的，但在 ASLR 环境中仍具有不可预测性，因为 `&__stack_chk_guard` 的地址是随机的。

### 系统算法

采用"最佳努力"初始化策略：

1. 若可获取内核随机熵，通过 `core::ptr::copy_nonoverlapping` 从 `AT_RANDOM` 缓冲区复制 —— 产生密码学安全的 canary。
2. 若不可获取，使用 `地址 × 大奇数常数` 生成伪随机值 —— ASLR 下的次优方案。
3. 在 64 位平台上，无论采用哪种方案，都将第二字节清零。端序由编译器自动处理（无需显式字节序判断）。
4. 最后将最终值写入线程 TLS 头部，确保线程安全。

Rust 内部实现可利用的安全抽象：
- 当 `entropy` 非 NULL 时，通过原始指针构造 `&[u8]` 切片后使用 `copy_nonoverlapping`。
- 使用 `cfg(target_pointer_width = "64")` 进行条件编译，替代 C 的 `UINTPTR_MAX` 宏判断。
- 使用 `const` 定义常量 `CANARY_MULTIPLIER: usize = 0x41C64E6D`。

### 线程安全
单线程调用；不需要同步原语。

### 常量定义

```rust
/// canary 确定性生成算法乘数常量
const CANARY_MULTIPLIER: usize = 0x41C64E6D;
```

---

## `__stack_chk_fail` (编译器 ABI 函数)

```rust
#[no_mangle]
extern "C" fn __stack_chk_fail() -> !;
```

[Visibility]: Internal — GCC/Clang 编译器 ABI 所需的栈破坏回调函数。不通过任何标准 C/POSIX 头文件对外声明。当编译器插入的栈保护桩代码检测到 canary 值不匹配时自动调用。用户程序不应直接调用此函数。

### 意图

当栈缓冲区溢出破坏了 canary 值时，函数在返回前检测到篡改并调用 `__stack_chk_fail`。此函数的目的不是优雅地报告错误，而是**立即、不可恢复地终止进程**，以防止攻击者利用栈溢出劫持控制流。

Rust 中返回类型使用 `!`（never 类型）明确表达"此函数永不返回"的语义，编译器可利用此信息进行优化和死代码消除。

### 前置条件
- 栈 canary 完整性检查失败（被调用者上下文中的 canary 值与 `__stack_chk_guard` 不匹配）。
- 栈帧可能已损坏，因此不能信任调用栈的完整性。

### 后置条件
- **此函数不返回**（Rust 类型 `!` 保证该语义）。
- 进程终止：通过 `unsafe { core::ptr::write_volatile(core::ptr::null_mut::<u8>(), 0); }` 执行空指针写入，触发硬件级别的 SIGSEGV。等价于 C 的 `a_crash()` 行为，但直接内联实现，无需跨模块依赖。
- 不调用任何 `atexit` 处理器，不执行标准 I/O 清理，不做任何可能被攻击者利用的操作。

### 系统算法
使用 `core::ptr::write_volatile` 对空指针地址写入 0，触发 SIGSEGV 信号终止进程。
- `write_volatile` 防止编译器优化掉该写操作（等价于 C 的 `volatile` 限定符）。
- 不使用 `core::intrinsics::abort()`（不稳定 API），仅依赖稳定 `core` API。
- 写入后使用 `unsafe { core::hint::unreachable_unchecked() }` 或 `loop {}` 满足 `!` 返回类型的控制流要求（若平台未在写入时立即终止）。

### 线程安全
无状态修改；纯终止操作。可在任何线程上下文中调用。

---

## `__stack_chk_fail_local` (DSO 内部符号)

```rust
#[no_mangle]
#[doc(hidden)]
extern "C" fn __stack_chk_fail_local() -> !;
```

[Visibility]: Internal — 编译器在某些平台（特别是 i386 老式 PLT 场景）生成的代码可能引用此局部符号而非 `__stack_chk_fail`。不是任何标准接口。

### 符号语义需求

`__stack_chk_fail_local` 必须满足以下两个语义属性：

1. **弱符号 (Weak Symbol)**: 若可执行文件或前置加载的共享库已定义 `__stack_chk_fail_local`，则该定义优先，rusl 的定义被覆盖。
2. **隐藏可见性 (Hidden Visibility)**: 该符号不对外导出 —— 仅在静态库内部解析。避免通过 PLT/GOT 的间接调用开销。

### Rust 实现策略

Rust 本身不直接支持 C 的 `weak_alias` 宏机制，可采用以下方案之一实现等价行为：

- **方案 A（推荐）**: 将 `__stack_chk_fail_local` 定义为直接调用 `__stack_chk_fail()` 的独立函数，依赖 LTO（链接时优化）消除调用开销。对于未启用 LTO 的场景，该额外跳转开销可忽略不计（因为此函数仅在进程崩溃时调用）。
- **方案 B**: 使用 `core::arch::global_asm!` 直接生成 ELF `.weak` + `.hidden` + `.set` 指令创建真正的零开销符号别名。此方案产生与 C 完全一致的目标代码，但依赖平台特定的汇编语法。
- **方案 C（nightly）**: 在稳定版 Rust 不推荐。使用 `#[linkage = "weak"]` 属性。

### 前置/后置条件
与 `__stack_chk_fail` 完全相同 —— 调用该函数最终触发进程终止，永不返回。

---

## 平台条件编译

```rust
use core::ptr;

const CANARY_MULTIPLIER: usize = 0x41C64E6D;

// 64 位平台：清零 canary 第二字节以防御字符串攻击
#[cfg(target_pointer_width = "64")]
fn apply_canary_mask(canary: usize) -> usize {
    canary & !0xFF00  // 将第二字节置零 (小端平台等效)
}

// 32 位平台：使用原始 canary 值（无第二字节清零）
#[cfg(not(target_pointer_width = "64"))]
fn apply_canary_mask(canary: usize) -> usize {
    canary  // 保持不变
}
```

利用 `cfg(target_pointer_width)` 条件编译替代 C 代码中的 `#if UINTPTR_MAX > 0xffffffff` 宏判断，由编译器在编译时静态选择正确的代码路径，无运行时开销。

---

## 跨模块依赖

| 符号 | 来源 | 关系 |
|------|------|------|
| `__pthread_self` | rusl 内部 pthread 模块 | `__init_ssp` 获取线程控制块引用以同步 canary |

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ptr::copy_nonoverlapping::<u8>   // 依赖1: 稳定 no_std 内存复制原语，替代 C 的 memcpy
  core::ptr::write_volatile::<u8>        // 依赖2: 稳定 no_std volatile 写入原语，用于空指针崩溃（替代 C 的 a_crash）
  core::hint::unreachable_unchecked      // 依赖3: 稳定 no_std 不可达路径标定，用于满足 ! 返回类型（可选，也可用 loop {}）
  fn __pthread_self() -> &mut Pthread;   // 依赖4: 获取当前线程控制块的可变引用，用于写入 canary 字段

Note:
  - a_crash() 不列入依赖：__stack_chk_fail 直接内联 core::ptr::write_volatile 实现崩溃，
    消除对 src/internal/atomic 模块的跨文件依赖，减少耦合。
  - core::intrinsics::abort 不列入依赖：使用稳定 core API 替代不稳定 intrinsic。

[GUARANTEE]
Exported Interface (Compiler ABI Symbols):
  #[no_mangle] static mut __stack_chk_guard: usize = 0;
                                  // 编译器 ABI: 全局栈 canary 变量，GCC/Clang 生成的栈保护代码直接引用
  #[no_mangle] extern "C" fn __stack_chk_fail() -> !;
                                  // 编译器 ABI: 栈破坏回调，canary 校验失败时编译器生成的代码自动调用
  #[no_mangle] extern "C" fn __stack_chk_fail_local() -> !;
                                  // 编译器 ABI: __stack_chk_fail 的 DSO 局部别名（某些平台的 PLT 优化路径）
