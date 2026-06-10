# sprintf — Rust 接口归约

## 复杂度分级: Level 1

> musl libc 字符串格式化输出函数（无边界检查）。是 `vsprintf(s, ...)` 的可变参数包装。纯转发代理。

---

## 原始 C 接口
```c
int sprintf(char *restrict s, const char *restrict fmt, ...);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

---

## Rust 外部 ABI 接口

```rust
// C ABI 兼容: sprintf 是可变参数函数
extern "C" {
    fn sprintf(
        s: *mut core::ffi::c_char,
        fmt: *const core::ffi::c_char,
        ...
    ) -> core::ffi::c_int;
}
```

推荐方案：`sprintf` 由 C 源码实现为 thin wrapper（调用 Rust 实现的 `vsprintf`）。

---

## Rust 安全接口设计

```rust
// Rust 原生的 sprintf 等价物——安全的可变缓冲区写入
pub fn rust_sprintf(buf: &mut [u8], fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
```

**注意**：`sprintf` 无边界检查，用户必须确保缓冲区足够大。`rust_sprintf` 同样不执行截断（与 `snprintf` 不同），调用者保证 `buf` 容量足够。行为等价于 `rust_vsprintf(buf, fmt, args)`。

---

## 意图

将格式化字符串写入用户提供的缓冲区 `s`。不执行边界检查，用户必须确保缓冲区足够大以容纳完整输出。

## 前置条件

- `s` 指向足够大的可写缓冲区（由调用者保证，无自动截断）
- `fmt != NULL`，指向有效的格式化字符串
- 可变参数与格式串匹配

## 后置条件

- Case 1 成功：返回写入 `s` 的字符总数（不含 `'\0'`），`s` 以 `'\0'` 结尾
- Case 2 失败：返回负值
- `va_list` 在返回前已通过 `va_end` 清理
- 行为等价于 `vsnprintf(s, INT_MAX, fmt, ap)`

## 不变量

无。本函数纯粹作为转发代理。

## 算法

```
sprintf(s, fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vsprintf(s, fmt, ap) 委托内部实现（等价于 vsnprintf(s, INT_MAX, fmt, ap)）
  3. va_end(ap) 清理
  4. return ret
```

Rust 实现的 `rust_sprintf` 直接调用 `rust_vsprintf`，无需 `va_list` 中间层。

对于 C ABI 兼容性，推荐与 musl 原始设计一致的方案——`sprintf` 由 C 源文件实现作为 thin wrapper：

```c
// 辅助 C 文件（sprintf_cabi.c）
#include <stdarg.h>
#include <stdio.h>

int sprintf(char *s, const char *fmt, ...) {
    int ret;
    va_list ap;
    va_start(ap, fmt);
    ret = vsprintf(s, fmt, ap);
    va_end(ap);
    return ret;
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  int vsprintf(char *s, const char *fmt, va_list ap);
                                   // 依赖1: C ABI vsprintf 实现
  pub(crate) fn rust_vsprintf(buf: &mut [u8], fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                   // 依赖2: Rust 内部格式化引擎
  pub(crate) enum FormatArg { ... }
                                   // 依赖3: 格式化参数类型（来自 vsnprintf 模块）

[GUARANTEE]
Exported Interface:
  extern "C" fn sprintf(
      s: *mut core::ffi::c_char,
      fmt: *const core::ffi::c_char,
      ...
  ) -> core::ffi::c_int;
                                 // 由 C 源码实现 thin wrapper
Internal Interface:
  pub fn rust_sprintf(buf: &mut [u8], fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                 // 安全的 Rust 原生格式化接口（无边界检查）
