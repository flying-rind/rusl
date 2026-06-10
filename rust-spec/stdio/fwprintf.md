# fwprintf 函数规约

## 复杂度分级: Level 1

> musl libc 宽字符格式化文件流输出函数。是 `vfwprintf(f, ...)` 的可变参数包装。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};
use crate::internal::FILE;

extern "C" fn fwprintf(f: *mut FILE, fmt: *const c_uint /* const wchar_t */, ...) -> c_int;
```

[Visibility]: `fwprintf` 是 `<wchar.h>` 标准库函数，对外导出。

Rust 侧实现策略：
- 使用 `va_list` 机制初始化可变参数列表
- 直接委托给 `vfwprintf(f, fmt, ap)`
- 返回前通过 `va_end` 清理
- 注意：Rust 的 `extern "C"` 可变参数函数使用 C ABI 兼容的可变参数机制

---

### 前置/后置条件

**[Pre-condition]:**
- `f` 指向有效的 `FILE` 对象
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- 可变参数与格式串匹配

**[Post-condition]:**
- Case 1 成功：返回写入 `f` 的宽字符总数
- Case 2 输出错误：返回 `-1`
- Case 3 格式错误：返回 `-1`，`errno = EINVAL`
- Case 4 溢出：返回 `-1`，`errno = EOVERFLOW`
- `va_list` 在返回前已通过 `va_end` 清理

**[Error Behavior]:**
- 格式错误时 `errno = EINVAL`
- 溢出时 `errno = EOVERFLOW`

---

### 不变量

无。本函数纯粹作为转发代理。

---

### 意图

将格式化宽字符串输出到指定的 `FILE` 流 `f`。是 `vfwprintf` 的可变参数包装器。与 `fprintf` 的区别在于格式字符串和输出均为宽字符。

---

### 系统算法

```
fwprintf(f, fmt, ...):
  1. va_start(ap, fmt)
  2. ret = vfwprintf(f, fmt, ap)
  3. va_end(ap)
  4. return ret
```

时间复杂度取决于格式串和参数数量。

---

## 依赖图

```
fwprintf (Public)
  └── vfwprintf(f, fmt, ap)  (see vfwprintf.c spec)
```

---

## [RELY]

- `vfwprintf` — 宽字符格式化输出核心引擎 (见 `vfwprintf.md`)
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏

## [GUARANTEE]

Exported Interface:
  `extern "C" fn fwprintf(f: *mut FILE, fmt: *const c_uint, ...) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号。
