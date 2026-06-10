# vwscanf 函数规约

## 复杂度分级: Level 1

> musl libc 宽字符标准输入格式化函数（va_list 版本）。直接委托给 `vfwscanf(stdin, ...)`。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};

// vwscanf — User, 标准库函数
extern "C" fn vwscanf(
    fmt: *const c_uint, /* const wchar_t */
    ap: va_list
) -> c_int;

// __isoc99_vwscanf — Internal (weak_alias -> vwscanf)
extern "C" fn __isoc99_vwscanf(fmt: *const c_uint, ap: va_list) -> c_int;
```

[Visibility]: `vwscanf` 是 `<stdarg.h>` / `<wchar.h>` 标准库函数，对外导出。`__isoc99_vwscanf` 为 Internal 符号（C99 兼容别名），通过弱别名对外导出。

Rust 侧实现策略：
- 直接委托给 `vfwscanf(stdin, fmt, ap)`
- 作为转发代理，实现极为简单

---

### 前置/后置条件

**[Pre-condition]:**
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- `ap` 由 `va_start` 正确初始化
- `stdin` 已初始化，可读取

**[Post-condition]:**
- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达 EOF）：返回 `EOF`
- Case 3 格式错误：返回已成功匹配的项数

**[Error Behavior]:**
- 输入失败时返回 `EOF`

---

### 不变量

无。本函数纯粹作为转发代理。

---

### 意图

从标准输入流 `stdin` 读取宽字符格式化输入。是 `wscanf` 的 `va_list` 版本。直接委托给 `vfwscanf(stdin, fmt, ap)`。

---

### 系统算法

```
vwscanf(fmt, ap):
  return vfwscanf(stdin, fmt, ap)
```

时间复杂度 O(1)（转发）。

---

## 依赖图

```
vwscanf (Public)
  └── vfwscanf(stdin, fmt, ap)  (see vfwscanf.c spec)

__isoc99_vwscanf (weak_alias)
  └── vwscanf

stdin (全局变量, 来自 <stdio.h>)
```

---

## [RELY]

- `vfwscanf` — 宽字符格式化输入核心引擎 (见 `vfwscanf.md`)
- `stdin` — 标准输入流 (见 `__stdin_used.md`)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn vwscanf(fmt: *const c_uint, ap: va_list) -> c_int;`
  `extern "C" fn __isoc99_vwscanf(fmt: *const c_uint, ap: va_list) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号。`__isoc99_vwscanf` 为 `vwscanf` 的弱别名，行为完全一致。
