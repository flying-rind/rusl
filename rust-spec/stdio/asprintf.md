# asprintf — Rust 接口归约

## 复杂度分级: Level 1

> musl libc 自动分配缓冲区的格式化输出函数。是 `vasprintf(s, ...)` 的可变参数包装（GNU 扩展）。纯转发代理。

---

## 原始 C 接口
```c
int asprintf(char **s, const char *fmt, ...);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出（GNU 扩展 / POSIX）

---

## Rust 外部 ABI 接口

```rust
// C ABI 兼容: asprintf 是可变参数函数，Rust 无法在 extern "C" fn 定义中使用 ...
// 采用 extern "C" 声明块，符号由 C 侧 thin wrapper 或链接器提供
extern "C" {
    fn asprintf(
        s: *mut *mut core::ffi::c_char,
        fmt: *const core::ffi::c_char,
        ...
    ) -> core::ffi::c_int;
}
```

推荐方案：`asprintf` 由 C 源码实现为 thin wrapper（调用 Rust 实现的 `vasprintf`），与 musl 原始设计一致。

---

## Rust 安全接口设计

```rust
// Rust 原生的 asprintf 等价物——返回String（堆分配）
pub fn rust_asprintf(fmt: &str, args: &[FormatArg]) -> Result<RustString, FmtError>;
```

此处 `RustString` 是 rusl 的 `no_std` String 类型，`FormatArg` 由 `vasprintf` 模块提供。`rust_asprintf` 作为薄包装：
1. 将 `&[FormatArg]` 直接传递给 `rust_vasprintf(fmt, args)`
2. 返回堆分配的格式化结果

---

## 意图

将格式化字符串写入动态分配的缓冲区。缓冲区由 `malloc` 分配，调用者负责 `free`。是 `vasprintf` 的可变参数包装器。

## 前置条件

- `s != NULL`，`*s` 的值将被覆盖
- `fmt != NULL`，指向有效的格式化字符串
- 可变参数与格式串匹配

## 后置条件

- Case 1 成功：
  - `*s` 指向 `malloc` 分配的缓冲区，包含 null 结尾的格式化字符串
  - 返回值为格式化字符串的长度（不含 `'\0'`）
  - 调用者有责任 `free(*s)`
- Case 2 失败（格式错误或分配失败）：返回 `-1`，`*s` 不变
- `va_list` 在返回前已通过 `va_end` 清理

## 不变量

无。本函数纯粹作为转发代理。

## 算法

```
asprintf(s, fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vasprintf(s, fmt, ap) 委托内部实现
  3. va_end(ap) 清理
  4. return ret
```

Rust 实现的 `rust_asprintf` 直接调用 `rust_vasprintf`，无需 `va_list` 中间层。

对于 C ABI 兼容性，推荐与 musl 原始设计一致的方案——`asprintf` 由 C 源文件实现作为 thin wrapper（调用 Rust 的 `vasprintf`）：

```c
// 辅助 C 文件（asprintf_cabi.c）
#include <stdarg.h>
#include <stdio.h>

int asprintf(char **s, const char *fmt, ...) {
    int ret;
    va_list ap;
    va_start(ap, fmt);
    ret = vasprintf(s, fmt, ap);
    va_end(ap);
    return ret;
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  int vasprintf(char **s, const char *fmt, va_list ap);
                                   // 依赖1: C ABI vasprintf 实现
  pub(crate) fn rust_vasprintf(fmt: &str, args: &[FormatArg]) -> Result<RustString, FmtError>;
                                   // 依赖2: Rust 内部格式化引擎
  pub(crate) enum FormatArg { ... }
                                   // 依赖3: 格式化参数类型（来自 vasprintf 模块）
  pub(crate) struct RustString { ... }
                                   // 依赖4: no_std String 类型（堆分配）

[GUARANTEE]
Exported Interface:
  // 方式: 由 C 侧 thin wrapper 提供 C ABI 符号，调用 Rust 的 vasprintf
  extern "C" fn asprintf(
      s: *mut *mut core::ffi::c_char,
      fmt: *const core::ffi::c_char,
      ...
  ) -> core::ffi::c_int;
                                 // 声明于 extern "C" 块中，由 C 源码实现
Internal Interface:
  pub fn rust_asprintf(fmt: &str, args: &[FormatArg]) -> Result<RustString, FmtError>;
                                 // 安全的 Rust 原生格式化接口
