# swscanf 函数规约

## 复杂度分级: Level 1

> musl libc 宽字符串格式化输入函数。是 `vswscanf(s, fmt, ...)` 的可变参数包装。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};

// swscanf — User, 标准库函数
extern "C" fn swscanf(
    s: *const c_uint,   /* const wchar_t */
    fmt: *const c_uint, /* const wchar_t */
    ...
) -> c_int;

// __isoc99_swscanf — Internal (weak_alias -> swscanf)
extern "C" fn __isoc99_swscanf(s: *const c_uint, fmt: *const c_uint, ...) -> c_int;
```

[Visibility]: `swscanf` 是 `<wchar.h>` 标准库函数，对外导出。`__isoc99_swscanf` 为 Internal 符号（C99 兼容别名），通过弱别名对外导出。

Rust 侧实现策略：
- 使用 `va_list` 机制初始化可变参数列表
- 直接委托给 `vswscanf(s, fmt, ap)`
- 返回前通过 `va_end` 清理

---

### 前置/后置条件

**[Pre-condition]:**
- `s != NULL`，指向以 `L'\0'` 结尾的有效宽字符串输入源
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- 可变参数与格式串匹配（指针类型参数必须指向有效位置）

**[Post-condition]:**
- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达字符串末尾）：返回 `EOF`
- `va_list` 在返回前已通过 `va_end` 清理

**[Error Behavior]:**
- 输入失败时返回 `EOF`

---

### 不变量

无。本函数纯粹作为转发代理。

---

### 意图

从宽字符串 `s` 读取格式化输入。是 `vswscanf` 的可变参数包装器。与 `sscanf` 的区别在于输入字符串和格式字符串均为宽字符。

---

### 系统算法

```
swscanf(s, fmt, ...):
  1. va_start(ap, fmt)
  2. ret = vswscanf(s, fmt, ap)
  3. va_end(ap)
  4. return ret
```

---

## 依赖图

```
swscanf (Public)
  └── vswscanf(s, fmt, ap)  (see vswscanf.c spec)

__isoc99_swscanf (weak_alias)
  └── swscanf
```

---

## [RELY]

- `vswscanf` — `va_list` 版宽字符串格式化输入 (见 `vswscanf.md`)
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏

## [GUARANTEE]

Exported Interface:
  `extern "C" fn swscanf(s: *const c_uint, fmt: *const c_uint, ...) -> c_int;`
  `extern "C" fn __isoc99_swscanf(s: *const c_uint, fmt: *const c_uint, ...) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号。`__isoc99_swscanf` 为 `swscanf` 的弱别名，行为完全一致。
