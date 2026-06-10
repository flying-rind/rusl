# printf — Rust 接口归约

## 复杂度分级: Level 1

> musl libc 标准输出格式化函数。是 `vfprintf(stdout, ...)` 的可变参数包装。纯转发代理。

---

## 原始 C 接口
```c
int printf(const char *restrict fmt, ...);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

---

## Rust 外部 ABI 接口

```rust
// C ABI 兼容: printf 是可变参数函数
extern "C" {
    fn printf(fmt: *const core::ffi::c_char, ...) -> core::ffi::c_int;
}
```

推荐方案：`printf` 由 C 源码实现为 thin wrapper（调用 Rust 实现的 `vfprintf`，绑定 `stdout`）。

---

## Rust 安全接口设计

```rust
// Rust 原生的 printf 等价物——输出到 stdout
pub fn rust_printf(fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
```

`rust_printf` 作为薄包装：将 `&[FormatArg]` 直接传递给 `rust_vfprintf(stdout, fmt, args)`。

---

## 意图

将格式化字符串输出到标准输出流 `stdout`。是 `vfprintf` 的可变参数包装器。

## 前置条件

- `fmt != NULL`，指向有效的格式化字符串
- `stdout` 已初始化，可写入
- 可变参数与格式串匹配

## 后置条件

- Case 1 成功：返回写入 `stdout` 的字符总数
- Case 2 输出错误：返回 `-1`
- Case 3 格式错误：返回 `-1`，`errno = EINVAL`
- Case 4 溢出：返回 `-1`，`errno = EOVERFLOW`
- `va_list` 在返回前已通过 `va_end` 清理

## 不变量

无。本函数纯粹作为转发代理。

## 算法

```
printf(fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vfprintf(stdout, fmt, ap) 委托核心引擎
  3. va_end(ap) 清理
  4. return ret
```

对于 C ABI 兼容性，推荐与 musl 原始设计一致的方案——`printf` 由 C 源文件实现作为 thin wrapper：

```c
// 辅助 C 文件（printf_cabi.c）
#include <stdarg.h>
#include <stdio.h>

int printf(const char *fmt, ...) {
    int ret;
    va_list ap;
    va_start(ap, fmt);
    ret = vfprintf(stdout, fmt, ap);
    va_end(ap);
    return ret;
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  int vfprintf(FILE *f, const char *fmt, va_list ap);
                                   // 依赖1: C ABI vfprintf 实现（核心引擎）
  FILE *stdout;                      // 依赖2: 标准输出流
  pub(crate) fn rust_vfprintf(f: &mut RustFile, fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                   // 依赖3: Rust 内部格式化引擎
  pub(crate) enum FormatArg { ... }
                                   // 依赖4: 格式化参数类型（来自 vfprintf 模块）

[GUARANTEE]
Exported Interface:
  extern "C" fn printf(fmt: *const core::ffi::c_char, ...) -> core::ffi::c_int;
                                 // 由 C 源码实现 thin wrapper
Internal Interface:
  pub fn rust_printf(fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                 // 安全的 Rust 原生格式化接口
