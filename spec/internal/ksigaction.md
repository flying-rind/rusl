# ksigaction.h 规约

## 概述

`ksigaction.h` 定义了与 Linux 内核 `rt_sigaction` 系统调用直接交互的 `struct k_sigaction` 结构体，以及信号处理函数返回后恢复上下文所需的 `__restore` / `__restore_rt` 函数声明。此结构是用户态 `struct sigaction`（POSIX）与内核态 `struct sigaction`（Linux）之间的桥梁——musl 在调用 `rt_sigaction` 系统调用时，将 POSIX 格式的信号动作转换为内核格式。

## 依赖图

```
ksigaction.h
├── <features.h>      (标准库头文件, 提供 SA_RESTORER 等架构相关宏)
│
├── struct k_sigaction (定义于本文件)
│   └── 依赖: 依赖架构顶级目录中的同名文件 (arch/<arch>/ksigaction.h) 可能覆写
│       无其他 musl 内部结构依赖
│
├── __restore()        (声明于本文件)
└── __restore_rt()     (声明于本文件)
```

## 类型/结构依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `SA_RESTORER` | `<features.h>` 或架构头文件 | 标准库宏，跳过 |
| `void (*)(int)` | C 内建函数指针类型 | 跳过 |
| `unsigned long` | C 内建类型 | 跳过 |
| `unsigned` | C 内建类型 | 跳过 |

---

## 结构体规约

### struct k_sigaction

```c
struct k_sigaction {
    void (*handler)(int);
    unsigned long flags;
#ifdef SA_RESTORER
    void (*restorer)(void);
#endif
    unsigned mask[2];
#ifndef SA_RESTORER
    void *unused;
#endif
};
```
```rust
// Rust
#[repr(C)]
pub struct KSigAction {
    pub handler: Option<unsafe extern "C" fn(c_int)>,
    pub flags: c_ulong,
    #[cfg(feature = "sa_restorer")]
    pub restorer: Option<unsafe extern "C" fn()>,
    pub mask: [c_uint; 2],
    #[cfg(not(feature = "sa_restorer"))]
    pub unused: *mut c_void,
}
```

[Visibility]: Internal (不导出) — musl 内部使用，仅在 `sigaction` 系统调用封装层中用于与 Linux 内核 `rt_sigaction` 交互。POSIX 标准定义的 `struct sigaction` 有不同布局，用户程序不应使用此结构。

**Intent**: Linux 内核的 `rt_sigaction` 系统调用期望的信号动作结构体布局与 POSIX 用户态 `struct sigaction` 不同。`k_sigaction` 充当适配层：
- `handler` 字段：与 POSIX 相同，信号处理函数指针（`SIG_DFL`/`SIG_IGN`/用户函数）
- `flags` 字段：与 POSIX 相同，`SA_*` 标志位集合
- `mask[2]` 字段：内核使用固定 2 个 `unsigned long`（共 64 位）存储信号掩码，而 POSIX 的 `sigset_t` 可能是 128 字节
- `restorer` / `unused` 字段：架构相关
  - 若架构定义了 `SA_RESTORER`（如 x86）：包含 `restorer` 函数指针，指向 sigreturn 蹦床
  - 若架构未定义 `SA_RESTORER`（如 aarch64）：该 8 字节填充为 `unused`

---

#### 字段契约

| 字段 | 条件编译 | 契约 |
|------|---------|------|
| `handler` | 无条件 | 信号处理函数指针。`(void(*)(int))0` = `SIG_DFL`，`(void(*)(int))1` = `SIG_IGN` |
| `flags` | 无条件 | 位掩码，包含 `SA_SIGINFO`, `SA_RESTART`, `SA_ONSTACK`, `SA_NODEFER`, `SA_RESETHAND` 等 |
| `restorer` | `SA_RESTORER` 定义时 | 指向 sigreturn trampoline（`__restore_rt`），内核在信号处理返回后跳转到此地址恢复上下文 |
| `mask[2]` | 无条件 | 信号掩码，内核格式：每个位对应一个信号编号（1-64） |
| `unused` | `SA_RESTORER` 未定义时 | 填充，大小 = `sizeof(void*)`，保持结构体布局一致 |

---

#### 布局不变量 (Layout Invariants)

1. **大小一致性**: `sizeof(struct k_sigaction)` 必须与 Linux 内核 `include/linux/signal_types.h` 中的结构体大小完全一致，否则 `rt_sigaction` 系统调用将失败。
2. **联合语义**: `restorer` 和 `unused` 在同一偏移量上互斥（由 `SA_RESTORER` 控制），它们实际共享同一内存位置。
3. **架构覆写**: 若架构顶级目录（如 `arch/x86_64/ksigaction.h`）存在同名文件，其定义将覆写此默认定义。

---

## 函数规约

### __restore

```c
hidden void __restore(void);
hidden void __restore_rt(void);
```
```rust
// Rust — 这两个函数极其特殊：
// 1. 它们永远不会被正常调用（不做 call），内核将它们地址写入 sigframe
// 2. 它们是信号处理返回时内核跳转到的 trampoline
// 3. Rust 中应当使用 global_asm! 定义，而非普通 fn

// 仅声明其存在性：
extern "C" {
    fn __restore();
    fn __restore_rt();
}
```

[Visibility]: Internal (不导出) — musl 内部的信号恢复蹦床（sigreturn trampoline）。这两个函数极其特殊：它们**永远不会被正常 C 代码调用**，而是由内核在信号处理返回时直接跳转到它们的地址。用户程序绝不可调用它们。

**Intent**: 当 Linux 内核调用用户态信号处理函数时，在调用前会在用户栈上压入一个"信号帧"（sigframe），其中包含恢复上下文所需的所有寄存器状态。信号处理函数执行 `ret` 指令后，需要执行 `rt_sigreturn` 系统调用来恢复原始上下文。但信号处理函数本身不能直接发起系统调用——需要通过一个中间"蹦床"函数。

`__restore` / `__restore_rt` 就是这两个蹦床函数：
- `__restore`: 用于旧式 `sigaction`（`SA_SIGINFO` 标志未设置时），内部执行 `sigreturn` 系统调用
- `__restore_rt`: 用于 `rt_sigaction`（`SA_SIGINFO` 标志设置时），内部执行 `rt_sigreturn` 系统调用

---

#### 极特殊契约

**前置条件**: N/A — 此函数永远不会被用户代码以正常调用约定调用。它存在的唯一目的是其**地址**被写入 `k_sigaction.restorer` 字段。

**后置条件**: N/A — 此函数不返回（它执行 `sigreturn` / `rt_sigreturn` 系统调用，该系统调用将寄存器状态恢复到信号到达前的状态，并从原始被中断位置继续执行）。

**Invariant**: 
1. `__restore` 和 `__restore_rt` 的地址在进程生命周期内不变（它们在文本段中）。
2. 若架构未定义 `SA_RESTORER`（如 aarch64），内核通过 VDSO 提供自己的恢复机制，这两个函数的声明存在但实际不使用——信号恢复由内核在 VDSO 中提供的 `__kernel_rt_sigreturn` 地址完成。

**实现要点** (Rust):
- 必须使用 `global_asm!` 实现，包含 `rt_sigreturn` 系统调用指令
- 不允许包含函数序言/尾声（prologue/epilogue），因为它们不在正常调用约定下执行
- 必须标记为 `#[naked]`（若 Rust 支持）或使用纯汇编，确保编译器不添加任何额外的栈帧操作