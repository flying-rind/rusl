# sscanf — Rust 接口归约

## 复杂度分级: Level 1

> musl libc 字符串格式化输入函数。是 `vsscanf(s, ...)` 的可变参数包装。纯转发代理。

---

## 原始 C 接口
```c
int sscanf(const char *restrict s, const char *restrict fmt, ...);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

---

## Rust 外部 ABI 接口

```rust
// C ABI 兼容: sscanf 是可变参数函数
extern "C" {
    fn sscanf(
        s: *const core::ffi::c_char,
        fmt: *const core::ffi::c_char,
        ...
    ) -> core::ffi::c_int;
}
```

推荐方案：`sscanf` 由 C 源码实现为 thin wrapper（调用 Rust 实现的 `vsscanf`）。

---

## Rust 弱别名（C99 兼容）

```rust
// weak_alias: __isoc99_sscanf 是 sscanf 的弱别名
extern "C" {
    fn __isoc99_sscanf(
        s: *const core::ffi::c_char,
        fmt: *const core::ffi::c_char,
        ...
    ) -> core::ffi::c_int;
}
```

[Visibility]: `sscanf` 为 User 导出符号，`__isoc99_sscanf` 为 Internal 符号（与 `sscanf` 行为完全一致）。

---

## Rust 安全接口设计

```rust
// Rust 原生的 sscanf 等价物——从内存字符串读取格式化输入
pub fn rust_sscanf(input: &[u8], fmt: &str, args: &mut [FormatDest]) -> Result<usize, ScanError>;
```

`rust_sscanf` 作为薄包装：将 `&mut [FormatDest]` 直接传递给 `rust_vsscanf(input, fmt, args)`。

---

## 意图

从内存中的 null 结尾字符串 `s` 读取格式化输入。是 `vsscanf` 的可变参数包装器。

## 前置条件

- `s` 指向有效的 null 结尾 C 字符串
- `fmt != NULL`，指向有效的格式化字符串
- 可变参数与格式串匹配（指针类型参数必须指向有效位置）

## 后置条件

- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达字符串结尾）：返回 `EOF`
- `s` 源字符串不会被修改
- `va_list` 在返回前已通过 `va_end` 清理

## 不变量

无。本函数纯粹作为转发代理。

## 算法

```
sscanf(s, fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vsscanf(s, fmt, ap) 委托内部实现
  3. va_end(ap) 清理
  4. return ret
```

对于 C ABI 兼容性，推荐方案——`sscanf` 由 C 源文件实现作为 thin wrapper：

```c
// 辅助 C 文件（sscanf_cabi.c）
#include <stdarg.h>
#include <stdio.h>

int sscanf(const char *s, const char *fmt, ...) {
    int ret;
    va_list ap;
    va_start(ap, fmt);
    ret = vsscanf(s, fmt, ap);
    va_end(ap);
    return ret;
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  int vsscanf(const char *s, const char *fmt, va_list ap);
                                   // 依赖1: C ABI vsscanf 实现
  pub(crate) fn rust_vsscanf(input: &[u8], fmt: &str, args: &mut [FormatDest]) -> Result<usize, ScanError>;
                                   // 依赖2: Rust 内部格式化引擎
  pub(crate) enum FormatDest { ... }
                                   // 依赖3: 格式化目标类型（来自 vfscanf 模块）

[GUARANTEE]
Exported Interface:
  extern "C" fn sscanf(
      s: *const core::ffi::c_char,
      fmt: *const core::ffi::c_char,
      ...
  ) -> core::ffi::c_int;
                                 // 由 C 源码实现 thin wrapper
  extern "C" fn __isoc99_sscanf(
      s: *const core::ffi::c_char,
      fmt: *const core::ffi::c_char,
      ...
  ) -> core::ffi::c_int;
                                 // C99 兼容弱别名
Internal Interface:
  pub fn rust_sscanf(input: &[u8], fmt: &str, args: &mut [FormatDest]) -> Result<usize, ScanError>;
                                 // 安全的 Rust 原生格式化输入接口
