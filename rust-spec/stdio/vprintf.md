# vprintf — Rust 接口归约

## 复杂度分级: Level 1

> musl libc `va_list` 版标准输出格式化函数。直接委托 `vfprintf(stdout, ...)`。纯转发代理。

---

## 原始 C 接口
```c
int vprintf(const char *restrict fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

---

## Rust 外部 ABI 接口

```rust
// C ABI 兼容: va_list 通过 core::ffi::VaList 传递
extern "C" fn vprintf(
    fmt: *const core::ffi::c_char,
    ap: core::ffi::VaList,
) -> core::ffi::c_int;
```

注意：Rust 的 `VaList` 类型（`core::ffi::VaList`）允许在 `extern "C"` 函数间传递 `va_list`。但构造 `VaList` 仍需通过 C 侧的 `va_start`，因此本函数的调用者通常是 C 侧的可变参数包装器（如 `printf`）。

---

## Rust 安全接口设计

```rust
// Rust 原生的 vprintf 等价物——直接使用格式化引擎输出到 stdout
pub fn rust_vprintf(fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
```

`rust_vprintf` 直接调用 `rust_vfprintf(stdout, fmt, args)` 格式化到标准输出流。

---

## 意图

将格式化字符串输出到标准输出流 `stdout`（`va_list` 版本）。是 `printf` 的 `va_list` 平替。

## 前置条件

- `fmt != NULL`，指向有效的格式化字符串
- `ap` 已由 `va_start` 正确初始化
- `stdout` 已初始化，可写入

## 后置条件

- Case 1 成功：返回写入 `stdout` 的字符总数
- Case 2 输出错误：返回 `-1`
- Case 3 格式错误：返回 `-1`，`errno = EINVAL`
- Case 4 溢出：返回 `-1`，`errno = EOVERFLOW`

## 不变量

无。本函数纯粹作为转发代理。

## 算法

```
vprintf(fmt, ap):
  return vfprintf(stdout, fmt, ap)
```

Rust 实现：
```
fn vprintf(fmt: *const c_char, ap: VaList) -> c_int {
    vfprintf(stdout_ptr, fmt, ap)  // 通过内部映射获取 stdout 的 *mut FILE
}
```

`rust_vprintf` 直接调用 `rust_vfprintf(stdout, fmt, args)`。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  int vfprintf(FILE *f, const char *fmt, va_list ap);
                                   // 依赖1: C ABI vfprintf 实现（核心引擎）
  FILE *stdout;                      // 依赖2: 标准输出流
  core::ffi::VaList                  // 依赖3: Rust 内置 va_list 类型
  pub(crate) fn rust_vfprintf(f: &mut RustFile, fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                   // 依赖4: Rust 内部格式化引擎
  pub(crate) enum FormatArg { ... }
                                   // 依赖5: 格式化参数类型（来自 vfprintf 模块）

[GUARANTEE]
Exported Interface:
  extern "C" fn vprintf(
      fmt: *const core::ffi::c_char,
      ap: core::ffi::VaList,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 vprintf 符号
Internal Interface:
  pub fn rust_vprintf(fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                 // 安全的 Rust 原生格式化接口
