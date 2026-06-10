# swprintf 函数规约

## 复杂度分级: Level 1

> musl libc 宽字符字符串格式化输出函数。是 `vswprintf(s, n, fmt, ...)` 的可变参数包装。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};

extern "C" fn swprintf(
    s: *mut c_uint /* wchar_t */,
    n: usize,            /* size_t */
    fmt: *const c_uint,  /* const wchar_t */
    ...
) -> c_int;
```

[Visibility]: `swprintf` 是 `<wchar.h>` 标准库函数，对外导出。

Rust 侧实现策略：
- 使用 `va_list` 机制初始化可变参数列表
- 直接委托给 `vswprintf(s, n, fmt, ap)`
- 返回前通过 `va_end` 清理
- 注意：`n` 的类型为 `size_t`（`usize`），`s` 和 `fmt` 为宽字符指针

---

### 前置/后置条件

**[Pre-condition]:**
- `s`: 指向有效宽字符缓冲区的指针（`n > 0` 时）；`n == 0` 时可为 `NULL`
- `n`: 缓冲区大小（宽字符数）
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- 可变参数与格式串匹配

**[Post-condition]:**
- Case 1 成功（输出未截断）：返回写入 `s` 的宽字符数（不含 `L'\0'`）
- Case 2 截断（`ret >= n`）：返回 `-1`（musl 行为，非 C99 标准）
- Case 3 `n == 0`：返回 `-1`
- Case 4 输出错误：返回 `-1`
- Case 5 格式错误：返回 `-1`，`errno = EINVAL`
- Case 6 溢出：返回 `-1`，`errno = EOVERFLOW`
- `s` 被 `L'\0'` 终止（当 `n > 0` 时）
- `va_list` 在返回前已通过 `va_end` 清理

**[Error Behavior]:**
- 截断时返回 `-1`（musl 特有行为）
- 格式错误时 `errno = EINVAL`
- 溢出时 `errno = EOVERFLOW`

---

### 不变量

无。本函数纯粹作为转发代理。

---

### 意图

将格式化宽字符串输出到缓冲区 `s`，最多写入 `n` 个宽字符（含终止 `L'\0'`）。是 `vswprintf` 的可变参数包装器。与 `snprintf` 类似，但格式字符串和目标缓冲区均为宽字符。

musl 行为注意：截断时返回 `-1` 而非截断前的完整长度，这与 C99 标准要求不同。

---

### 系统算法

```
swprintf(s, n, fmt, ...):
  1. va_start(ap, fmt)
  2. ret = vswprintf(s, n, fmt, ap)
  3. va_end(ap)
  4. return ret
```

---

## 依赖图

```
swprintf (Public)
  └── vswprintf(s, n, fmt, ap)  (see vswprintf.c spec)
```

---

## [RELY]

- `vswprintf` — 宽字符字符串格式化输出核心引擎 (见 `vswprintf.md`)
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏

## [GUARANTEE]

Exported Interface:
  `extern "C" fn swprintf(s: *mut c_uint, n: usize, fmt: *const c_uint, ...) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号。
