# vwprintf 函数规约

## 复杂度分级: Level 1

> musl libc 宽字符标准输出格式化函数（va_list 版本）。直接委托给 `vfwprintf(stdout, ...)`。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};

extern "C" fn vwprintf(
    fmt: *const c_uint, /* const wchar_t */
    ap: va_list
) -> c_int;
```

[Visibility]: `vwprintf` 是 `<stdarg.h>` / `<wchar.h>` 标准库函数，对外导出。

Rust 侧实现策略：
- 直接委托给 `vfwprintf(stdout, fmt, ap)`
- 作为转发代理，实现极为简单

---

### 前置/后置条件

**[Pre-condition]:**
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- `ap` 由 `va_start` 正确初始化
- `stdout` 已初始化，可写入

**[Post-condition]:**
- Case 1 成功：返回写入 `stdout` 的宽字符总数
- Case 2 输出错误：返回 `-1`
- Case 3 格式错误：返回 `-1`，`errno = EINVAL`
- Case 4 溢出：返回 `-1`，`errno = EOVERFLOW`

**[Error Behavior]:**
- 格式错误时 `errno = EINVAL`
- 溢出时 `errno = EOVERFLOW`

---

### 不变量

无。本函数纯粹作为转发代理。

---

### 意图

将格式化宽字符串输出到标准输出流 `stdout`。是 `wprintf` 的 `va_list` 版本。直接委托给 `vfwprintf(stdout, fmt, ap)`。

---

### 系统算法

```
vwprintf(fmt, ap):
  return vfwprintf(stdout, fmt, ap)
```

时间复杂度 O(1)（转发）。

---

## 依赖图

```
vwprintf (Public)
  └── vfwprintf(stdout, fmt, ap)  (see vfwprintf.c spec)

stdout (全局变量, 来自 <stdio.h>)
```

---

## [RELY]

- `vfwprintf` — 宽字符格式化输出核心引擎 (见 `vfwprintf.md`)
- `stdout` — 标准输出流 (见 `__stdout_used.md`)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn vwprintf(fmt: *const c_uint, ap: va_list) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号。
