# vsprintf — Rust 接口归约

## 复杂度分级: Level 1

> musl libc 字符串格式化输出函数（`va_list` 版本，无边界检查）。通过将 `INT_MAX` 传给 `vsnprintf` 实现。纯转发代理。

---

## 原始 C 接口
```c
int vsprintf(char *restrict s, const char *restrict fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

---

## Rust 外部 ABI 接口

```rust
// C ABI 兼容: va_list 通过 core::ffi::VaList 传递
extern "C" fn vsprintf(
    s: *mut core::ffi::c_char,
    fmt: *const core::ffi::c_char,
    ap: core::ffi::VaList,
) -> core::ffi::c_int;
```

---

## Rust 安全接口设计

```rust
// Rust 原生的 vsprintf 等价物——无边界检查的缓冲区写入
pub fn rust_vsprintf(buf: &mut [u8], fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
```

`rust_vsprintf` 等价于 `rust_vsnprintf(buf, usize::MAX, fmt, args)`，即传入最大 size 参数调用 `rust_vsnprintf`。

---

## 意图

将格式化字符串写入用户提供的缓冲区 `s`（`va_list` 版本）。不执行边界检查。

## 前置条件

- `s` 指向足够大的可写缓冲区（调用者保证）
- `fmt != NULL`，指向有效的格式化字符串
- `ap` 已由 `va_start` 正确初始化

## 后置条件

- Case 1 成功：返回写入 `s` 的字符总数（不含 `'\0'`），`s` 以 `'\0'` 结尾
- Case 2 失败：返回负值
- 行为等价于 `vsnprintf(s, INT_MAX, fmt, ap)`

## 不变量

无。本函数纯粹作为转发代理（`INT_MAX` 作为 size 参数传入 `vsnprintf`）。

## 算法

```
vsprintf(s, fmt, ap):
  return vsnprintf(s, INT_MAX, fmt, ap)
```

Rust 实现：
```
fn vsprintf(s: *mut c_char, fmt: *const c_char, ap: VaList) -> c_int {
    vsnprintf(s, INT_MAX, fmt, ap)
}
```

`rust_vsprintf` 直接调用 `rust_vsnprintf(buf, usize::MAX, fmt, args)`。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  int vsnprintf(char *s, size_t n, const char *fmt, va_list ap);
                                   // 依赖1: C ABI vsnprintf 实现
  core::ffi::c_int::MAX              // 依赖2: INT_MAX 值（作为无边界检查的 sentinel）
  core::ffi::VaList                  // 依赖3: Rust 内置 va_list 类型
  pub(crate) fn rust_vsnprintf(buf: &mut [u8], fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                   // 依赖4: Rust 内部格式化引擎
  pub(crate) enum FormatArg { ... }
                                   // 依赖5: 格式化参数类型（来自 vsnprintf 模块）

[GUARANTEE]
Exported Interface:
  extern "C" fn vsprintf(
      s: *mut core::ffi::c_char,
      fmt: *const core::ffi::c_char,
      ap: core::ffi::VaList,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 vsprintf 符号
Internal Interface:
  pub fn rust_vsprintf(buf: &mut [u8], fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                 // 安全的 Rust 原生格式化接口（无边界检查）
