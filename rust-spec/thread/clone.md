# clone — Rust 接口归约

> rusl 内部 `clone` 系统调用的占位实现。在 musl 中，`clone` 的实际实现位于架构相关的汇编文件中（如 `x86_64/clone.s`），此 C 文件提供返回 `-ENOSYS` 的默认回退实现。

## 原始 C 接口

```c
int __clone(int (*func)(void *), void *stack, int flags, void *arg, ...);
```

[Visibility]: Internal — 被 `pthread_impl.h` 声明为 hidden，仅 musl/rusl 内部使用。实际实现由架构相关的汇编文件提供。

---

## Rust 外部 ABI 接口

```rust
pub extern "C" fn __clone(
    func: Option<extern "C" fn(*mut core::ffi::c_void) -> core::ffi::c_int>,
    stack: *mut core::ffi::c_void,
    flags: core::ffi::c_int,
    arg: *mut core::ffi::c_void,
    // 可变参数部分 (如 ptid, ctid, newtls, ...) 通过架构相关的 ABI 传递
    ...
) -> core::ffi::c_int;
```

---

## 依赖图

```
__clone (占位实现) → 返回 -ENOSYS (无实际依赖)

__clone (架构相关汇编实现, 如 x86_64/clone.s)
  └─> linux syscall: SYS_clone           (Linux clone 系统调用)
```

---

## 函数规约

### 1. __clone

```rust
pub extern "C" fn __clone(
    func: Option<extern "C" fn(*mut core::ffi::c_void) -> core::ffi::c_int>,
    stack: *mut core::ffi::c_void,
    flags: core::ffi::c_int,
    arg: *mut core::ffi::c_void,
    ...
) -> core::ffi::c_int;
```

#### Intent

此 C 源文件是 `clone` 系统调用的占位实现。在正常情况下，此文件被架构相关的汇编实现（`arch/<arch>/clone.s`）替代，后者通过 `SYS_clone` 系统调用创建新线程。

此版本的意图是提供一个安全的回退：在未提供汇编实现的架构上，返回 `-ENOSYS` 表示不支持。

#### 前置条件

- 无（在接受任何参数前即返回 `-ENOSYS`）

#### 后置条件

- 始终返回 `-ENOSYS`（功能未实现）

#### 系统算法

```
__clone(func, stack, flags, arg, ...):
  1. return -ENOSYS
```

#### 不变量

- 此函数永不创建线程 -- 始终返回 `-ENOSYS`
- 架构相关的汇编实现（如 `arch/x86_64/clone.s`）在链接时替代此符号

---

## Rust 实现说明

在 rusl 中，`__clone` 的 Rust 实现可以有两种策略：

### 策略一：汇编回退（与 musl 一致）

保持此文件为纯回退实现（返回 `-ENOSYS`），实际的 `clone` 由架构相关的汇编文件提供。这适合对 musl 的逐函数替代。

### 策略二：Rust 内联汇编实现

使用 `core::arch::asm!` 直接在 Rust 中实现 `clone` 系统调用，避免依赖外部汇编文件。由于 `clone` 涉及新线程的栈设置和函数调用约定，这需要仔细的内联汇编处理。

```rust
#[cfg(target_arch = "x86_64")]
pub extern "C" fn __clone(
    func: Option<extern "C" fn(*mut c_void) -> c_int>,
    stack: *mut c_void,
    flags: c_int,
    arg: *mut c_void,
    ptid: *mut c_int,
    ctid: *mut c_int,
    newtls: *mut c_void,
) -> c_int {
    unsafe {
        // 使用 asm! 实现 clone 系统调用
        // 子线程中: 调用 func(arg), 退出
        // 父线程中: 返回子线程 TID
        // ...
    }
}
```

> 推荐使用策略一（汇编回退）以保持与 musl 的一致性。仅在需要减少汇编依赖时考虑策略二。

#### 依赖

- `ENOSYS` — 错误码（定义于 errno）
- `linux syscall: SYS_clone` — clone 系统调用（仅在实际汇编实现中使用）

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 clone 抽象（架构相关的汇编实现替代回退后使用）
// 此包装不在占位文件中，仅供架构相关模块内部参考

use core::ffi::{c_int, c_void};

/// 创建新线程的内部安全抽象
/// 在支持 clone 的架构上调用底层的 clone 系统调用
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "riscv64"))]
pub(crate) fn clone_thread(
    func: extern "C" fn(*mut c_void) -> c_int,
    stack_base: *mut c_void,
    flags: c_int,
    arg: *mut c_void,
) -> Result<c_int, c_int> {
    // 调用架构相关的 __clone 实现
    let ret = unsafe { /* __clone(...) */ };
    if ret < 0 { Err(-ret) } else { Ok(ret) }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  errno::ENOSYS                           // 依赖1: 不支持错误码
  (若使用架构汇编实现) arch/<arch>/clone.s  // 依赖2: 架构相关汇编
  linux syscall: SYS_clone                // 依赖3: clone 系统调用

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __clone(func: Option<extern "C" fn(*mut c_void) -> c_int>,
                             stack: *mut c_void, flags: c_int, arg: *mut c_void, ...) -> c_int;
                                         // 本模块保证对外提供与 C ABI 兼容的 __clone 符号
Internal Interface:
  pub(crate) fn clone_thread(func: extern "C" fn(*mut c_void) -> c_int,
                              stack_base: *mut c_void, flags: c_int, arg: *mut c_void)
                              -> Result<c_int, c_int>;
                                         // 安全包装，供 crate 内部使用（仅在架构实现替代回退后可用）
