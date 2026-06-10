# vscanf — Rust 接口归约

## 复杂度分级: Level 1

> musl libc `va_list` 版标准输入格式化读取函数。直接委托 `vfscanf(stdin, ...)`。纯转发代理。

---

## 原始 C 接口
```c
int vscanf(const char *restrict fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

---

## Rust 外部 ABI 接口

```rust
// C ABI 兼容: va_list 通过 core::ffi::VaList 传递
extern "C" fn vscanf(
    fmt: *const core::ffi::c_char,
    ap: core::ffi::VaList,
) -> core::ffi::c_int;
```

## Rust 弱别名（C99 兼容）

```rust
// weak_alias: __isoc99_vscanf 是 vscanf 的弱别名
extern "C" fn __isoc99_vscanf(
    fmt: *const core::ffi::c_char,
    ap: core::ffi::VaList,
) -> core::ffi::c_int;
```

[Visibility]: `vscanf` 为 User 导出符号，`__isoc99_vscanf` 为 Internal 符号（与 `vscanf` 行为完全一致）。

---

## Rust 安全接口设计

```rust
// Rust 原生的 vscanf 等价物——从 stdin 读取
pub fn rust_vscanf(fmt: &str, args: &mut [FormatDest]) -> Result<usize, ScanError>;
```

`rust_vscanf` 直接调用 `rust_vfscanf(stdin, fmt, args)` 从标准输入流读取。

---

## 意图

从标准输入流 `stdin` 读取格式化输入（`va_list` 版本）。是 `scanf` 的 `va_list` 平替。

## 前置条件

- `fmt != NULL`，指向有效的格式化字符串
- `ap` 已由 `va_start` 正确初始化
- `stdin` 已初始化，可读取

## 后置条件

- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达 EOF）：返回 `EOF`
- Case 3 格式错误：返回已成功匹配的项数

## 不变量

无。本函数纯粹作为转发代理。

## 算法

```
vscanf(fmt, ap):
  return vfscanf(stdin, fmt, ap)
```

Rust 实现：
```
fn vscanf(fmt: *const c_char, ap: VaList) -> c_int {
    vfscanf(stdin_ptr, fmt, ap)  // 通过内部映射获取 stdin 的 *mut FILE
}
```

`rust_vscanf` 直接调用 `rust_vfscanf(stdin, fmt, args)`。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  int vfscanf(FILE *f, const char *fmt, va_list ap);
                                   // 依赖1: C ABI vfscanf 实现（核心引擎）
  FILE *stdin;                       // 依赖2: 标准输入流
  core::ffi::VaList                  // 依赖3: Rust 内置 va_list 类型
  pub(crate) fn rust_vfscanf(f: &mut RustFile, fmt: &str, args: &mut [FormatDest]) -> Result<usize, ScanError>;
                                   // 依赖4: Rust 内部格式化引擎
  pub(crate) enum FormatDest { ... }
                                   // 依赖5: 格式化目标类型（来自 vfscanf 模块）

[GUARANTEE]
Exported Interface:
  extern "C" fn vscanf(
      fmt: *const core::ffi::c_char,
      ap: core::ffi::VaList,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 vscanf 符号
  extern "C" fn __isoc99_vscanf(
      fmt: *const core::ffi::c_char,
      ap: core::ffi::VaList,
  ) -> core::ffi::c_int;
                                 // C99 兼容弱别名
Internal Interface:
  pub fn rust_vscanf(fmt: &str, args: &mut [FormatDest]) -> Result<usize, ScanError>;
                                 // 安全的 Rust 原生格式化输入接口
