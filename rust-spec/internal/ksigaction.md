# ksigaction 规约 (Rust)

## 概述

`ksigaction` 模块定义了与 Linux 内核 `rt_sigaction` 系统调用直接交互的 `KSigAction` 结构体，以及信号处理函数返回后恢复上下文所需的 `__restore` / `__restore_rt` 蹦床函数。此结构是用户态 `struct sigaction`(POSIX) 与内核态 `struct sigaction`(Linux) 之间的桥梁——rusl 在调用 `rt_sigaction` 系统调用时，将 POSIX 格式的信号动作转换为内核格式。

## 依赖图

```
ksigaction 模块
├── 架构条件编译宏 (SA_RESTORER, 定义于架构相关 features 模块)
│
├── struct KSigAction (定义于本模块, #[repr(C)])
│   └── 依赖: 可能被架构顶级目录的同名定义覆写
│
├── __restore()        (使用 global_asm! 定义)
└── __restore_rt()     (使用 global_asm! 定义)
```

---

```
/* Rely */
[RELY]
架构依赖:
  SA_RESTORER 宏 (条件编译)                  // 依赖1: 架构特性宏, 决定 restorer/unused 字段
  内核 rt_sigaction 系统调用 ABI             // 依赖2: 内核期望的结构体布局

内部依赖:
  rt_sigreturn 系统调用号                    // 依赖3: __restore_rt 蹦床中使用的系统调用号

[GUARANTEE]
内部接口:
  #[repr(C)] pub(crate) struct KSigAction;  // 本模块保证: 与内核 struct sigaction 布局一致
  global_asm! { __restore: ... }            // 本模块保证: __restore 地址可写入 restorer 字段
  global_asm! { __restore_rt: ... }         // 本模块保证: __restore_rt 地址可写入 restorer 字段
```

---

## 结构体规约

### KSigAction

```rust
// Rust — 与 Linux 内核 rt_sigaction 系统调用交互的信号动作结构体
#[repr(C)]
pub(crate) struct KSigAction {
    pub handler: Option<unsafe extern "C" fn(c_int)>,  // 信号处理函数指针
    pub flags: c_ulong,                                 // SA_* 标志位集
    #[cfg(feature = "sa_restorer")]
    pub restorer: Option<unsafe extern "C" fn()>,       // 信号恢复蹦床指针
    pub mask: [c_uint; 2],                              // 信号掩码(内核格式, 64位)
    #[cfg(not(feature = "sa_restorer"))]
    pub unused: *mut c_void,                            // 对齐填充
}
```

[Visibility]: Internal (不导出) — rusl 内部使用，仅在 `sigaction` 系统调用封装层中用于与 Linux 内核 `rt_sigaction` 交互。POSIX 标准定义的 `struct sigaction` 有不同布局，用户程序不应使用此结构。

**Intent**: Linux 内核的 `rt_sigaction` 系统调用期望的信号动作结构体布局与 POSIX 用户态 `struct sigaction` 不同。`KSigAction` 充当适配层:
- `handler` 字段: 与 POSIX 相同，信号处理函数指针(`SIG_DFL`/`SIG_IGN`/用户函数)
- `flags` 字段: 与 POSIX 相同，`SA_*` 标志位集合
- `mask[2]` 字段: 内核使用固定 2 个 `c_uint`(共 64 位)存储信号掩码，而 POSIX 的 `sigset_t` 可能是 128 字节
- `restorer` / `unused` 字段: 架构相关
  - 若架构定义了 `SA_RESTORER`(如 x86): 包含 `restorer` 函数指针，指向 sigreturn 蹦床
  - 若架构未定义 `SA_RESTORER`(如 aarch64): 该 8 字节填充为 `unused`

---

#### 字段契约

| 字段 | 条件编译 | 契约 |
|------|---------|------|
| `handler` | 无条件 | 信号处理函数指针。`None` = `SIG_DFL`(以 null 指针表示), `Some(1 as fn)` = `SIG_IGN`(以地址 1 表示) |
| `flags` | 无条件 | 位掩码，包含 `SA_SIGINFO`, `SA_RESTART`, `SA_ONSTACK`, `SA_NODEFER`, `SA_RESETHAND` 等 |
| `restorer` | `cfg(feature = "sa_restorer")` | 指向 sigreturn trampoline(`__restore_rt`)，内核在信号处理返回后跳转到此地址恢复上下文 |
| `mask[2]` | 无条件 | 信号掩码，内核格式: 每个位对应一个信号编号(1-64) |
| `unused` | `cfg(not(feature = "sa_restorer"))` | 填充，大小 = `sizeof(usize)`，保持结构体布局一致 |

---

#### 布局不变量 (Layout Invariants)

1. **大小一致性**: `size_of::<KSigAction>()` 必须与 Linux 内核中的结构体大小完全一致，否则 `rt_sigaction` 系统调用将失败。
2. **联合语义**: `restorer` 和 `unused` 在同一偏移量上互斥(由条件编译控制)，它们实际共享同一内存位置。
3. **架构覆写**: 若架构顶级目录存在同名定义，其定义将覆写此默认定义。在 Rust 中通过 `#[cfg(target_arch = "...")]` 条件编译实现。
4. **ABI 兼容**: 结构体必须标记 `#[repr(C)]` 以确保与 C 端内核 ABI 完全一致。

---

## 函数规约 (蹦床)

### __restore 和 __restore_rt

```rust
// Rust — 这两个函数极其特殊:
// 1. 它们永远不会被正常调用(不做 call)，内核将它们地址写入 sigframe
// 2. 它们是信号处理返回时内核跳转到的 trampoline
// 3. 必须使用 global_asm! 定义，而非普通 fn

// 使用 global_asm! 定义蹦床(伪代码示意):
// global_asm!(
//     ".global __restore",
//     "__restore:",
//     "    mov $SYS_sigreturn, %eax",  // 架构相关
//     "    int $0x80",                  // 架构相关
//     ".global __restore_rt",
//     "__restore_rt:",
//     "    mov $SYS_rt_sigreturn, %eax",
//     "    syscall",
// );

// 声明其存在性(供链结器引用):
extern "C" {
    fn __restore();
    fn __restore_rt();
}
```

[Visibility]: Internal (不导出) — rusl 内部的信号恢复蹦床(sigreturn trampoline)。这两个函数极其特殊: 它们**永远不会被正常 Rust 代码调用**，而是由内核在信号处理返回时直接跳转到它们的地址。用户程序绝不可调用它们。

**Intent**: 当 Linux 内核调用用户态信号处理函数时，在调用前会在用户栈上压入一个"信号帧"(sigframe)，其中包含恢复上下文所需的所有寄存器状态。信号处理函数执行 `ret` 指令后，需要执行 `rt_sigreturn` 系统调用来恢复原始上下文。但信号处理函数本身不能直接发起系统调用——需要通过一个中间"蹦床"函数。

`__restore` / `__restore_rt` 就是这两个蹦床函数:
- `__restore`: 用于旧式 `sigaction`(`SA_SIGINFO` 标志未设置时)，内部执行 `sigreturn` 系统调用
- `__restore_rt`: 用于 `rt_sigaction`(`SA_SIGINFO` 标志设置时)，内部执行 `rt_sigreturn` 系统调用

---

#### 极特殊契约

**前置条件**: N/A — 此函数永远不会被用户代码以正常调用约定调用。它存在的唯一目的是其**地址**被写入 `KSigAction.restorer` 字段。

**后置条件**: N/A — 此函数不返回(它执行 `sigreturn` / `rt_sigreturn` 系统调用，该系统调用将寄存器状态恢复到信号到达前的状态，并从原始被中断位置继续执行)。

**Invariant**:
1. `__restore` 和 `__restore_rt` 的地址在进程生命周期内不变(它们在文本段中)。
2. 若架构未定义 `SA_RESTORER`(如 aarch64)，内核通过 VDSO 提供自己的恢复机制，这两个函数的声明存在但实际不使用——信号恢复由内核在 VDSO 中提供的 `__kernel_rt_sigreturn` 地址完成。

---

## 实现指南 (rusl/Rust)

- `KSigAction` 使用 `#[repr(C)]` 结构体，字段类型使用 `c_ulong`、`c_uint`、`c_int` 等 FFI 安全类型
- `handler` 字段在 Rust 中表示为 `Option<unsafe extern "C" fn(c_int)>`:
  - `None`(空指针) = `SIG_DFL`
  - `Some(unsafe { transmute::<usize, fn(c_int)>(1) })` = `SIG_IGN`
- 条件编译字段使用 `#[cfg(feature = "sa_restorer")]` / `#[cfg(not(feature = "sa_restorer"))]`
- `__restore` 和 `__restore_rt` 必须使用 `global_asm!` 实现，包含 `rt_sigreturn` 系统调用指令:
  - 不允许包含函数序言/尾声(prologue/epilogue)，因为它们不在正常调用约定下执行
  - 在支持 `#[naked]` 属性的平台可考虑使用，但 `global_asm!` 更精确可控
  - 必须确保编译器不添加任何额外的栈帧操作
- 使用 `core::arch::global_asm!` 而非 `std::arch::global_asm!`(no_std 环境)