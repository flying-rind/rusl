# fwscanf 函数规约

## 复杂度分级: Level 1

> musl libc 宽字符格式化文件流输入函数。是 `vfwscanf(f, ...)` 的可变参数包装。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};
use crate::internal::FILE;

// fwscanf — User, 标准库函数
extern "C" fn fwscanf(f: *mut FILE, fmt: *const c_uint /* const wchar_t */, ...) -> c_int;

// __isoc99_fwscanf — Internal (weak_alias -> fwscanf)
extern "C" fn __isoc99_fwscanf(f: *mut FILE, fmt: *const c_uint, ...) -> c_int;
```

[Visibility]: `fwscanf` 是 `<wchar.h>` 标准库函数，对外导出。`__isoc99_fwscanf` 为 Internal 符号（C99 兼容别名），通过弱别名对外导出。

Rust 侧实现策略：
- 使用 `va_list` 机制初始化可变参数列表
- 直接委托给 `vfwscanf(f, fmt, ap)`
- 返回前通过 `va_end` 清理

---

### 前置/后置条件

**[Pre-condition]:**
- `f` 指向有效的 `FILE` 对象
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- 可变参数与格式串匹配（指针类型参数必须指向有效位置）

**[Post-condition]:**
- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达 EOF 或编码错误）：返回 `EOF`（即 `WEOF`）
- `va_list` 在返回前已通过 `va_end` 清理

**[Error Behavior]:**
- 输入失败时返回 `EOF`

---

### 不变量

无。本函数纯粹作为转发代理。

---

### 意图

从 `FILE` 流 `f` 读取宽字符格式化输入。是 `vfwscanf` 的可变参数包装器。与 `fscanf` 的区别在于格式字符串为宽字符，且匹配的字符按宽字符处理。

---

### 系统算法

```
fwscanf(f, fmt, ...):
  1. va_start(ap, fmt)
  2. ret = vfwscanf(f, fmt, ap)
  3. va_end(ap)
  4. return ret
```

---

## 依赖图

```
fwscanf (Public)
  └── vfwscanf(f, fmt, ap)  (see vfwscanf.c spec)

__isoc99_fwscanf (weak_alias)
  └── fwscanf
```

---

## [RELY]

- `vfwscanf` — 宽字符格式化输入核心引擎 (见 `vfwscanf.md`)
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏

## [GUARANTEE]

Exported Interface:
  `extern "C" fn fwscanf(f: *mut FILE, fmt: *const c_uint, ...) -> c_int;`
  `extern "C" fn __isoc99_fwscanf(f: *mut FILE, fmt: *const c_uint, ...) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号。`__isoc99_fwscanf` 为 `fwscanf` 的弱别名，行为完全一致。
