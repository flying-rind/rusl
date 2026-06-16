# __syscall_cp — Rust 接口归约

> rusl 内部带取消点检查的系统调用包装器。在调用底层系统调用之前检查线程取消点，使得可被取消的系统调用（如 `read`、`write`、`futex wait` 等）在取消请求挂起时能正确响应。实际的取消点检查逻辑在架构相关的汇编实现中；C 源文件提供默认回退（直接调用 `__syscall`）。

## 原始 C 接口

```c
// sccp (static) — 默认回退实现
static long sccp(syscall_arg_t nr, syscall_arg_t u, syscall_arg_t v,
                 syscall_arg_t w, syscall_arg_t x, syscall_arg_t y, syscall_arg_t z);

// __syscall_cp (导出) — 带取消点的系统调用入口
long (__syscall_cp)(syscall_arg_t nr, syscall_arg_t u, syscall_arg_t v,
                    syscall_arg_t w, syscall_arg_t x, syscall_arg_t y, syscall_arg_t z);
```

[Visibility]:
- `sccp`：Internal (不导出) — 文件作用域静态函数，作为 `__syscall_cp_c` 的弱别名默认实现
- `__syscall_cp`：Internal — 被 `syscall.h` 声明为 hidden，仅 musl/rusl 内部使用

---

## Rust 外部 ABI 接口

```rust
/// 带取消点检查的系统调用入口 (C ABI 兼容)
/// 函数名使用 raw identifier `r#__syscall_cp` 避免与同名宏冲突
pub extern "C" fn __syscall_cp(
    nr: syscall_arg_t,
    u: syscall_arg_t,
    v: syscall_arg_t,
    w: syscall_arg_t,
    x: syscall_arg_t,
    y: syscall_arg_t,
    z: syscall_arg_t,
) -> core::ffi::c_long;
```

> 注意：musl 中函数名带有括号 `(__syscall_cp)` 以防止与同名宏展开冲突。Rust 中使用 `#[no_mangle]` 显式指定链接名为 `__syscall_cp`。

---

## 依赖图

```
__syscall_cp
  └─> __syscall_cp_c(nr, u, v, w, x, y, z)  (外部 — 架构相关汇编实现)

sccp_c (内部默认回退)
  └─> __syscall(nr, u, v, w, x, y, z)        (外部 — 原始系统调用宏)
```

---

## 函数规约

### 1. sccp_c (内部默认回退, 不对外导出)

```rust
// crate 内部可见，作为单线程构建的默认回退
pub(crate) fn sccp_c(
    nr: syscall_arg_t,
    u: syscall_arg_t,
    v: syscall_arg_t,
    w: syscall_arg_t,
    x: syscall_arg_t,
    y: syscall_arg_t,
    z: syscall_arg_t,
) -> core::ffi::c_long;
```

#### Intent

在单线程/无取消支持的构建中，作为 `__syscall_cp_c` 的默认实现。直接调用 `__syscall` 执行系统调用，不检查取消点。多线程时由架构相关的汇编实现覆盖。

#### 前置条件

- `nr` 为有效的系统调用号
- 其余参数为系统调用参数

#### 后置条件

- 直接委托给 `__syscall(nr, u, v, w, x, y, z)`
- 返回值为系统调用结果（原始返回值，含负的 errno）

#### 系统算法

```
sccp_c(nr, u, v, w, x, y, z):
  1. return __syscall(nr, u, v, w, x, y, z)
```

---

### 2. __syscall_cp (导出符号)

```rust
pub extern "C" fn __syscall_cp(
    nr: syscall_arg_t,
    u: syscall_arg_t,
    v: syscall_arg_t,
    w: syscall_arg_t,
    x: syscall_arg_t,
    y: syscall_arg_t,
    z: syscall_arg_t,
) -> core::ffi::c_long;
```

#### Intent

带取消点检查的系统调用入口。函数定义本身直接委托给 `__syscall_cp_c`。实际的取消点检查逻辑在架构相关的汇编实现 `__syscall_cp_c` 中（该汇编实现在调用系统调用前检查 `cancel` 标志，并在需要时调用取消处理）。

在 rusl 中，可考虑以下实现策略：
- 单线程回退：调用 `sccp_c` 直接执行系统调用
- 多线程构建：使用 `core::sync::atomic::AtomicI32` 检查全局取消标志，若被取消则调用取消处理，否则执行系统调用

#### 前置条件

- `nr` 为有效的系统调用号
- 参数与目标系统调用的 ABI 要求匹配

#### 后置条件

- 若线程有挂起的取消请求且取消未被禁用，在系统调用前执行取消处理
- 返回值为系统调用结果

#### 系统算法

```
__syscall_cp(nr, u, v, w, x, y, z):
  1. return __syscall_cp_c(nr, u, v, w, x, y, z)
```

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 内部使用的带取消点检查系统调用辅助函数
pub(crate) fn syscall_with_cancel_check(
    nr: syscall_arg_t,
    args: [syscall_arg_t; 6],
) -> core::ffi::c_long {
    // 检查取消标志，若被取消则处理取消
    // 否则执行系统调用
    // ...
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __syscall_cp_c()                        // 依赖1: 架构相关的实际实现（汇编）
  __syscall()                             // 依赖2: 原始系统调用宏
  syscall_arg_t                           // 依赖3: 系统调用参数类型

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __syscall_cp(nr: syscall_arg_t, u: syscall_arg_t, v: syscall_arg_t,
                                  w: syscall_arg_t, x: syscall_arg_t, y: syscall_arg_t,
                                  z: syscall_arg_t) -> c_long;
                                         // 本模块保证对外提供与 C ABI 兼容的 __syscall_cp 符号
Internal Interface:
  pub(crate) fn sccp_c(nr: syscall_arg_t, ...) -> c_long;
  pub(crate) fn syscall_with_cancel_check(nr: syscall_arg_t, args: [syscall_arg_t; 6]) -> c_long;
                                         // 安全包装，供 crate 内部使用
