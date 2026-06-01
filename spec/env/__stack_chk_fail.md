# __stack_chk_fail.c 规约

## 依赖图

```
__stack_chk_guard (全局变量)
  └── __init_ssp → memcpy    (libc, 外部模块)
                  → __pthread_self() → pthread_impl.h (内部头文件, 跨文件依赖)
                      └── .canary 字段写入

__stack_chk_fail → a_crash() (src/internal/atomic.h, 外部模块 — see atomic.h spec)

__stack_chk_fail_local ──(weak_alias)──→ __stack_chk_fail
```

---

## 模块概述

本文件实现了 GCC/Clang 栈保护器(Stack Smashing Protector, SSP)的运行时支持。当程序以 `-fstack-protector` 或类似选项编译时，编译器会在每个函数的栈帧中插入一个"canary"（金丝雀值），并在函数返回前检查该值是否被篡改。若检测到栈缓冲区溢出，则调用 `__stack_chk_fail` 终止程序。

musl 在以下环节提供 SSP 支持：
1. **Canary 初始化** (`__init_ssp`)：在程序启动早期，通过 AT_RANDOM 辅助向量提供的随机熵（或回退到确定性算法）初始化全局 canary 值 `__stack_chk_guard`。
2. **线程 Canary 同步**：将 `__stack_chk_guard` 的值复制到当前线程的 TLS 头部 `canary` 字段，确保每个线程都能正确访问 canary。
3. **栈破坏响应** (`__stack_chk_fail`)：当检测到 canary 被篡改时，通过空指针解引用触发 SIGSEGV 立即终止程序。

---

## `__stack_chk_guard` (全局变量)

```c
uintptr_t __stack_chk_guard;
```

[Visibility]: Internal — GCC/Clang 编译器 ABI 所需的栈保护 canary 全局变量，不通过任何标准 C/POSIX 头文件对外声明。编译器在生成栈保护代码时直接引用此符号。用户程序不应直接访问。

### 前置条件
- 无。该变量在程序加载时为零初始化的 BSS 段中。

### 后置条件
- 在 `__init_ssp()` 调用后，持有当前进程的栈 canary 值，该值在进程生命周期内保持不变。

### 不变量
- `__stack_chk_guard` 在 `__init_ssp()` 调用后始终非零。
- `__stack_chk_guard == __pthread_self()->canary`（初始化后始终成立）。
- 在 64 位平台 (`UINTPTR_MAX >= 0xffffffffffffffff`) 上，`((char *)&__stack_chk_guard)[1] == 0`：canary 的第二字节始终为零（NULL 字节），用于防御通过字符串操作函数进行的 canary 泄漏/覆盖。

### 意图
Canary 值的更新被设计为一次性初始化：`__init_ssp` 在程序启动早期调用一次，之后 `__stack_chk_guard` 仅被读取。这保证了 canary 值的不可变性，防止攻击者通过信息泄漏后覆写 canary。

---

## `__init_ssp` (内部函数)

```c
void __init_ssp(void *entropy);
```

[Visibility]: Internal — musl 内部栈保护初始化函数。不是 POSIX/C 标准接口，不在任何公开头文件中声明。由 musl 的 C 运行时启动代码（`__init_libc` 或等效入口）在程序启动早期调用。

### 意图

`__init_ssp` 是 musl 栈保护机制的初始化入口。它从内核提供的辅助向量（AUXV）中获取随机熵，生成每个进程唯一、不可预测的 canary 值。当无法获取随机熵时，回退到基于地址的确定性算法（仍然提供一定程度的不可预测性，因为 ASLR 使得 `&__stack_chk_guard` 的地址随机）。

此外，在 64 位平台上刻意将 canary 第二字节清零，以防御通过 `strcpy`/`sprintf` 等字符串操作函数泄漏或覆写 canary 的攻击：攻击者如果以字符串方式溢出缓冲区，会被 NULL 字节截断而无法完成覆盖。

### 前置条件

- 调用发生在程序启动的极早期阶段，在任何用户代码之前。
- 调用发生在单线程环境中（此时仅有主线程存在）。
- `entropy` 参数由启动代码设置：
  - 非 NULL：指向从内核 `AT_RANDOM` 辅助向量获得的随机字节缓冲区（至少 `sizeof(uintptr_t)` 字节）。
  - NULL：表示随机熵不可用（如在内核未提供 `AT_RANDOM` 的平台上）。

### 后置条件

**Case 1: 有随机熵 (`entropy != NULL`)**
- `__stack_chk_guard` 被设置为从 `entropy` 指向的缓冲区复制的 `sizeof(uintptr_t)` 字节值。
- 在 64 位平台上，第二字节 `((char *)&__stack_chk_guard)[1]` 被强制置零，牺牲 8 位熵以换取字符串攻击防御。
- 当前线程的 `canary` 字段被设置为 `__stack_chk_guard` 的值。

**Case 2: 无随机熵 (`entropy == NULL`)**
- `__stack_chk_guard` 被设置为 `(uintptr_t)&__stack_chk_guard * 1103515245`（即 `0x41C64E6D`，一个常见乘数常量的十六进制表示）。
- 在 64 位平台上，第二字节同样被清零。
- 当前线程的 `canary` 字段被同步设置。
- 虽然此值是确定性的，但在 ASLR 环境中仍具有不可预测性，因为 `&__stack_chk_guard` 的地址是随机的。

### 系统算法

采用"最佳努力"初始化策略：

1. 若可获取内核随机熵，直接从 `AT_RANDOM` 复制 —— 产生密码学安全的 canary。
2. 若不可获取，使用 `地址 × 大奇数常数` 生成伪随机值 —— ASLR 下的次优方案。
3. 在 64 位平台上，无论采用哪种方案，都将第二字节清零。端序由编译器自动处理（无需显式字节序判断）。
4. 最后将最终值写入线程 TLS 头部，确保线程安全。

### 线程安全
单线程调用；不需要同步原语。

---

## `__stack_chk_fail` (编译器 ABI 函数)

```c
void __stack_chk_fail(void);
```

[Visibility]: Internal — GCC/Clang 编译器 ABI 所需的栈破坏回调函数。不通过任何标准 C/POSIX 头文件对外声明。当编译器插入的栈保护桩代码检测到 canary 值不匹配时自动调用。用户程序不应直接调用此函数。

### 意图

当栈缓冲区溢出破坏了 canary 值时，函数在返回前检测到篡改并调用 `__stack_chk_fail`。此函数的目的不是优雅地报告错误，而是**立即、不可恢复地终止进程**，以防止攻击者利用栈溢出劫持控制流。通过空指针解引用触发 SIGSEGV 而非调用 `abort()`，是因为在栈已被破坏的情况下，`abort()` 的复杂处理（信号处理器、atexit 回调等）本身可能被攻击者利用。

### 前置条件
- 栈 canary 完整性检查失败（被调用者上下文中的 canary 值与 `__stack_chk_guard` 不匹配）。
- 栈帧可能已损坏，因此不能信任调用栈的完整性。

### 后置条件
- **此函数不返回**。
- 进程因 SIGSEGV 信号终止（空指针解引用 `*(volatile char *)0 = 0`）。
- 不调用任何 `atexit` 处理器，不执行标准 I/O 清理，不做任何可能被攻击者利用的操作。

### 系统算法
调用 `a_crash()`（定义于 `src/internal/atomic.h`），该函数执行 `*(volatile char *)0 = 0` 这一确定性的空指针解引用操作，触发硬件级别的段错误。`volatile` 限定符防止编译器优化掉该写操作。

### 线程安全
无状态修改；纯终止操作。可在任何线程上下文中调用。

---

## `__stack_chk_fail_local` (DSO 内部符号)

```c
hidden void __stack_chk_fail_local(void);
```

通过 `weak_alias(__stack_chk_fail, __stack_chk_fail_local)` 定义为 `__stack_chk_fail` 的弱别名。

[Visibility]: Internal — musl 内部实现，不对外导出。使用 `hidden` 可见性和 `weak_alias` 机制创建 `__stack_chk_fail` 的局部别名，用于 DSO（动态共享对象）内部符号解析。不是任何标准接口。

### 意图

在某些平台上（特别是 i386 老式 PLT 场景），GCC 生成的代码可能通过局部符号 `__stack_chk_fail_local` 而非 `__stack_chk_fail` 来引用栈保护失败函数。这对于避免通过 PLT/GOT 的间接调用开销以及支持隐藏可见性优化至关重要。musl 通过 `weak_alias` 宏定义为别名，等价于：

```c
extern __typeof(__stack_chk_fail) __stack_chk_fail_local
    __attribute__((__weak__, __alias__("__stack_chk_fail")));
```

### 前置/后置条件
与 `__stack_chk_fail` 完全相同，因为它是同一函数的别名。

---

## 外部依赖汇总

| 依赖项 | 来源模块 | 说明 |
|--------|----------|------|
| `memcpy` | libc (`<string.h>`) | 从内核熵缓冲区复制 canary 值 |
| `__pthread_self()` | musl 内部 (`src/internal/pthread_impl.h`) | 获取当前线程的 pthread 结构指针，用于写入 `canary` 字段 |
| `a_crash()` | musl 内部 (`src/internal/atomic.h`) | 通过空指针解引用终止进程。详见 `src/internal/spec/atomic.md` |
| `weak_alias` | musl 内部 (`src/include/features.h`) | 创建弱别名的宏 |
| `hidden` | musl 内部 (`src/include/features.h`) | 设置 ELF 隐藏可见性属性的宏 |
| `UINTPTR_MAX` | libc (`<stdint.h>`) | 编译时平台检测（32 位 vs 64 位） |