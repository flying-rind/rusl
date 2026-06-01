# snprintf — Rust 接口归约

## 原始 C 接口
```c
int snprintf(char *restrict s, size_t n, const char *restrict fmt, ...);
```

---

## Rust 外部 ABI 接口

```rust
// C ABI 兼容: snprintf 是可变参数函数，Rust 的 extern "C" 不支持可变参数
// 只能通过声明外部符号的方式由 C 链接器解析，Rust 侧不实现可变参数的 extern "C" 函数体
// 替代方案: 导出为非可变参数的包装，或由调用者通过 va_list 路径使用 vsnprintf
extern "C" {
    fn snprintf(s: *mut core::ffi::c_char, n: usize, fmt: *const core::ffi::c_char, ...) -> core::ffi::c_int;
}
```

注意：Rust 不支持在 `extern "C" fn` **定义**中使用 `...`（可变参数），只能在 `extern "C" {}` **声明块**中声明此类符号供 FFI 调用。若需要 Rust 侧实现 `snprintf`，必须：
1. 实现 `vsnprintf`（接收 `VaList`）
2. 将 `snprintf` 实现为 C 侧的 thin wrapper（或用宏 hack）

---

## Rust 安全接口设计

```rust
// Rust 原生的 snprintf 等价物——不使用 va_list
pub fn rust_snprintf(buf: &mut [u8], fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
```

此处 `FormatArg` 由 `vsnprintf` 模块提供，`rust_snprintf` 作为薄包装：
1. 将 `&[FormatArg]` 直接传递给 `rust_vsnprintf(buf, fmt, args)`
2. 返回格式化后的长度

---

## 意图
可变参数的 `snprintf`。Rust 侧的核心价值在于 **安全包装**——接受 Rust 原生类型切片替代 C 的 `va_list`，消除未定义行为风险。

## 前置条件
- 同 `rust_vsnprintf`：`buf.len() >= 1`，`fmt` 有效，`args` 类型匹配

## 后置条件
- 同 `rust_vsnprintf`

## 不变量
无。本函数纯粹作为转发代理。

## 算法
```
fn rust_snprintf(buf: &mut [u8], fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError> {
    rust_vsnprintf(buf, fmt, args)
}
```

对于 C ABI 兼容性：
- 在编译时，`snprintf` 符号可由 C 源文件提供（调用 `vsnprintf`）
- 或使用 `cbindgen` 自动生成 C 头文件，将 Rust 的 `vsnprintf` 包装为 `snprintf`

推荐方案：`snprintf` 由 C 源码实现作为 thin wrapper（调用 Rust 实现的 `vsnprintf`），与 musl 原始设计一致。

```c
// 辅助 C 文件（snprintf_cabi.c），作为 Rust 实现的桥接
#include <stdarg.h>
#include <stdio.h>

int snprintf(char *restrict s, size_t n, const char *restrict fmt, ...) {
    int ret;
    va_list ap;
    va_start(ap, fmt);
    ret = vsnprintf(s, n, fmt, ap);  // 调用 Rust 实现的 vsnprintf
    va_end(ap);
    return ret;
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  int vsnprintf(char *restrict s, size_t n, const char *restrict fmt, va_list ap);
                                   // 依赖1: C ABI vsnprintf 实现
  pub(crate) fn rust_vsnprintf(buf: &mut [u8], fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                   // 依赖2: Rust 内部格式化引擎
  pub(crate) enum FormatArg { ... }
                                   // 依赖3: 格式化参数类型（来自 vsnprintf 模块）
Predefined Macros:
  (none)                           // 纯代理，无宏依赖

[GUARANTEE]
Exported Interface:
  // 方式A: 由 C 侧 thin wrapper 提供 C ABI 符号，调用 Rust 的 vsnprintf
  extern "C" fn snprintf(s: *mut core::ffi::c_char, n: usize, fmt: *const core::ffi::c_char, ...) -> core::ffi::c_int;
                                 // 声明于 extern "C" 块中，由 C 源码实现
Internal Interface:
  pub fn rust_snprintf(buf: &mut [u8], fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                 // 安全的 Rust 原生格式化接口