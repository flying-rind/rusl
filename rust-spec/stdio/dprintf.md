# dprintf — Rust 接口归约

## 复杂度分级: Level 1

> musl libc 文件描述符格式化输出函数。是 `vdprintf(fd, ...)` 的可变参数包装（POSIX 扩展）。纯转发代理。

---

## 原始 C 接口
```c
int dprintf(int fd, const char *restrict fmt, ...);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出（POSIX 扩展）

---

## Rust 外部 ABI 接口

```rust
// C ABI 兼容: dprintf 是可变参数函数
extern "C" {
    fn dprintf(
        fd: core::ffi::c_int,
        fmt: *const core::ffi::c_char,
        ...
    ) -> core::ffi::c_int;
}
```

推荐方案：`dprintf` 由 C 源码实现为 thin wrapper（调用 Rust 实现的 `vdprintf`）。

---

## Rust 安全接口设计

```rust
// Rust 原生的 dprintf 等价物
pub fn rust_dprintf(fd: RawFd, fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
```

`rust_dprintf` 作为薄包装：将 `&[FormatArg]` 直接传递给 `rust_vdprintf(fd, fmt, args)`。

---

## 意图

将格式化字符串写入文件描述符 `fd`。是 `vdprintf` 的可变参数包装器。

## 前置条件

- `fd` 为有效的文件描述符
- `fmt != NULL`，指向有效的格式化字符串
- 可变参数与格式串匹配

## 后置条件

- Case 1 成功：返回写入 `fd` 的字符总数
- Case 2 输出错误：返回 `-1`
- Case 3 格式错误：返回 `-1`，`errno = EINVAL`
- Case 4 溢出：返回 `-1`，`errno = EOVERFLOW`
- `va_list` 在返回前已通过 `va_end` 清理

## 不变量

无。本函数纯粹作为转发代理。

## 算法

```
dprintf(fd, fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vdprintf(fd, fmt, ap) 委托内部实现
  3. va_end(ap) 清理
  4. return ret
```

对于 C ABI 兼容性，推荐与 musl 原始设计一致的方案——`dprintf` 由 C 源文件实现作为 thin wrapper：

```c
// 辅助 C 文件（dprintf_cabi.c）
#include <stdarg.h>
#include <stdio.h>

int dprintf(int fd, const char *fmt, ...) {
    int ret;
    va_list ap;
    va_start(ap, fmt);
    ret = vdprintf(fd, fmt, ap);
    va_end(ap);
    return ret;
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  int vdprintf(int fd, const char *fmt, va_list ap);
                                   // 依赖1: C ABI vdprintf 实现
  pub(crate) fn rust_vdprintf(fd: RawFd, fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                   // 依赖2: Rust 内部格式化引擎
  pub(crate) enum FormatArg { ... }
                                   // 依赖3: 格式化参数类型（来自 vfprintf 模块）

[GUARANTEE]
Exported Interface:
  extern "C" fn dprintf(
      fd: core::ffi::c_int,
      fmt: *const core::ffi::c_char,
      ...
  ) -> core::ffi::c_int;
                                 // 由 C 源码实现 thin wrapper
Internal Interface:
  pub fn rust_dprintf(fd: RawFd, fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                 // 安全的 Rust 原生格式化接口
