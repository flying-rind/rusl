# wscanf 函数规约

## 复杂度分级: Level 1

> musl libc 宽字符标准输入格式化函数。是 `vwscanf(fmt, ...)` 的可变参数包装，最终委托给 `vfwscanf(stdin, ...)`。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};

// wscanf — User, 标准库函数
extern "C" fn wscanf(fmt: *const c_uint /* const wchar_t */, ...) -> c_int;

// __isoc99_wscanf — Internal (weak_alias -> wscanf)
extern "C" fn __isoc99_wscanf(fmt: *const c_uint, ...) -> c_int;
```

[Visibility]: `wscanf` 是 `<wchar.h>` 标准库函数，对外导出。`__isoc99_wscanf` 为 Internal 符号（C99 兼容别名），通过弱别名对外导出。

Rust 侧实现策略：
- 使用 `va_list` 机制初始化可变参数列表
- 直接委托给 `vwscanf(fmt, ap)`（最终到 `vfwscanf(stdin, fmt, ap)`）
- 返回前通过 `va_end` 清理

---

### 前置/后置条件

**[Pre-condition]:**
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- 可变参数与格式串匹配（指针类型参数必须指向有效位置）
- `stdin` 已初始化，可读取

**[Post-condition]:**
- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达 EOF）：返回 `EOF`
- `va_list` 在返回前已通过 `va_end` 清理

**[Error Behavior]:**
- 输入失败时返回 `EOF`

---

### 不变量

无。本函数纯粹作为转发代理。

---

### 意图

从标准输入流 `stdin` 读取宽字符格式化输入。是 `vwscanf` 的可变参数包装器。与 `scanf` 的区别在于格式字符串为宽字符，且匹配的字符按宽字符处理。

---

### 系统算法

```
wscanf(fmt, ...):
  1. va_start(ap, fmt)
  2. ret = vwscanf(fmt, ap)
  3. va_end(ap)
  4. return ret
```

---

## 依赖图

```
wscanf (Public)
  └── vwscanf(fmt, ap)  (see vwscanf.c spec)
        └── vfwscanf(stdin, fmt, ap)  (see vfwscanf.c spec)

__isoc99_wscanf (weak_alias)
  └── wscanf
```

---

## [RELY]

- `vwscanf` — `va_list` 版标准输入宽字符格式化读取 (见 `vwscanf.md`)
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏

## [GUARANTEE]

Exported Interface:
  `extern "C" fn wscanf(fmt: *const c_uint, ...) -> c_int;`
  `extern "C" fn __isoc99_wscanf(fmt: *const c_uint, ...) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号。`__isoc99_wscanf` 为 `wscanf` 的弱别名，行为完全一致。
