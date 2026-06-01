# __shcall.rs 规约 (Rust)

> **来源 C spec**: `musl/src/internal/spec/__shcall.md`
> **对应源文件**: `musl/src/internal/sh/__shcall.c`
> **复杂度层级**: Level 1 — 纯转发包装器
> **所属模块**: rusl 内部 stdio 扫描辅助（SuperH 架构变体）

---

## 依赖图

```
__shcall ──> func(arg) (函数指针间接调用)
```

---

## 概述

`__shcall` 是 musl 在 SuperH (sh) 架构上的转发包装器，将函数指针调用从共享库 PLT 延迟绑定路径中解耦。

在 rusl（Rust 实现）中，由于 Rust 编译器不会产生 PLT 相关的重入问题，此函数作为透明转发器存在，编译器通常可以将其完全内联消除。若 rusl 不需要支持 SuperH 架构，此符号可以省略。

---

## 函数声明

### `__shcall(arg: *mut c_void, func: Option<unsafe extern "C" fn(*mut c_void) -> c_int>) -> c_int`

```rust
// Rust 签名
pub(crate) unsafe fn __shcall(
    arg: *mut core::ffi::c_void,
    func: Option<unsafe extern "C" fn(*mut core::ffi::c_void) -> core::ffi::c_int>,
) -> core::ffi::c_int
```

[Visibility]: Internal — 仅在需要 PLT 隔离的架构（如 SuperH）上使用。rusl 中可被编译器完全内联为直接调用 `func(arg)`。

### 意图 (Intent)

在需要 PLT 隔离的架构上，作为函数指针间接调用的转发层，避免 PLT 延迟绑定可能导致的信号处理重入问题。在 Rust 中，此问题不存在（Rust 不通过 PLT 调用），故 `__shcall` 在 rusl 中为透明的编译时消除包装。

### 前置条件

- `func` 不为 `None`（有效的函数指针）
- `arg` 的类型与 `func` 期望的参数类型兼容
- 若调用发生在信号处理上下文，`func` 必须是 async-signal-safe

### 后置条件

- 返回值等于 `func(arg)` 的返回值
- 函数无任何副作用（除 `func(arg)` 的副作用外）
- 此函数自身不修改 `errno`

### 不变量

`__shcall` 在任何架构上都是纯转发（恒等变换）：`__shcall(a, f) ≡ f(a)`。此不变量保证替换 `__shcall` 的语义与直接调用 `func(arg)` 完全等价。

### 系统算法 (System Algorithm)

**设计模式：间接调用解耦**

```
调用者 ──函数指针──> __shcall ──直接调用──> func(arg)
```

在 Rust 实现中，由于没有 PLT 概念，`__shcall` 就是一个带 `Option` 安全检查的函数指针调用转发。编译器在优化级别足够时会将其完全内联，使得最终代码等同于直接调用 `func(arg)`。

### Rust 设计要点

- 使用 `Option<unsafe extern "C" fn(...)>` 替代 C 的裸函数指针，提供编译期空指针检查
- 若确定为不必要的架构，可以直接省略此函数，使用方直接调用函数指针即可
- 标记为 `#[inline(always)]` 确保编译器将其内联消除

---

## 跨文件依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `core::ffi::c_void` | Rust core 库 | 无需追踪 |
| `core::ffi::c_int` | Rust core 库 | 无需追踪 |

---

## RELY / GUARANTEE

```
[RELY]
Rust Core 内建类型:
  core::ffi::c_void                // 依赖1: C void 类型
  core::ffi::c_int                 // 依赖2: C int 类型

[GUARANTEE]
pub(crate) 接口:
  unsafe fn __shcall(arg: *mut c_void, func: Option<unsafe extern "C" fn(*mut c_void) -> c_int>) -> c_int
                                   // 函数指针转发包装器（PLT 解耦）
```