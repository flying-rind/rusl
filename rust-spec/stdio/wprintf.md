# wprintf 函数规约

## 复杂度分级: Level 1

> musl libc 宽字符标准输出格式化函数。是 `vwprintf(fmt, ...)` 的可变参数包装，最终委托给 `vfwprintf(stdout, ...)`。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};

extern "C" fn wprintf(fmt: *const c_uint /* const wchar_t */, ...) -> c_int;
```

[Visibility]: `wprintf` 是 `<wchar.h>` 标准库函数，对外导出。

Rust 侧实现策略：
- 使用 `va_list` 机制初始化可变参数列表
- 直接委托给 `vwprintf(fmt, ap)`（最终到 `vfwprintf(stdout, fmt, ap)`）
- 返回前通过 `va_end` 清理

---

### 前置/后置条件

**[Pre-condition]:**
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- `stdout` 已初始化，可写入
- 可变参数与格式串匹配

**[Post-condition]:**
- Case 1 成功：返回写入 `stdout` 的宽字符总数
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

将格式化宽字符串输出到标准输出流 `stdout`。是 `vwprintf` 的可变参数包装器。与 `printf` 的区别在于格式字符串为宽字符。

---

### 系统算法

```
wprintf(fmt, ...):
  1. va_start(ap, fmt)
  2. ret = vwprintf(fmt, ap)
  3. va_end(ap)
  4. return ret
```

---

## 依赖图

```
wprintf (Public)
  └── vwprintf(fmt, ap)  (see vwprintf.c spec)
        └── vfwprintf(stdout, fmt, ap)  (see vfwprintf.c spec)
```

---

## [RELY]

- `vwprintf` — 宽字符标准输出格式化函数 (见 `vwprintf.md`)
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏

## [GUARANTEE]

Exported Interface:
  `extern "C" fn wprintf(fmt: *const c_uint, ...) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号。
