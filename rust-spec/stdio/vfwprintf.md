# vfwprintf 函数规约

## 复杂度分级: Level 3

> musl libc 宽字符格式化输出核心引擎。实现 `vfwprintf` 函数及所有内部辅助函数、状态机、类型系统。与 `vfprintf.c` 的结构高度对称，区别在于格式字符串和终端输出均为宽字符。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};
use crate::internal::FILE;

// vfwprintf — User, 标准库函数
extern "C" fn vfwprintf(
    f: *mut FILE,
    fmt: *const c_uint, /* const wchar_t */
    ap: va_list          // Rust 侧使用对应的 va_list 类型
) -> c_int;
```

[Visibility]: `vfwprintf` 是 `<stdarg.h>` / `<wchar.h>` 标准库函数，对外导出。

Rust 侧实现策略：
- `vfwprintf` 函数体保持 ABI 兼容，使用 `extern "C"` 和 C 类型
- 内部辅助函数（`wprintf_core`、`out`、`pad`、`pop_arg`、`getint`）均为模块私有函数
- 状态机表、sizeprefix 等为模块级 `static` 常量
- `union arg` 可用 Rust 的 `enum` 或 `union` 安全封装
- 格式标志位可使用 `bitflags!` 宏
- 对于数值类型（`%d`、`%f` 等），构建窄字符格式串委托给 `fprintf` 处理
- 宽字符特有类型（`%C`、`%S`）自行处理

---

### 内部类型定义（模块私有）

```rust
// 格式标志位 (模块私有)
use bitflags::bitflags;

bitflags! {
    struct FormatFlags: u32 {
        const ALT_FORM   = 1 << ('#' as u32 - ' ' as u32);
        const ZERO_PAD   = 1 << ('0' as u32 - ' ' as u32);
        const LEFT_ADJ   = 1 << ('-' as u32 - ' ' as u32);
        const PAD_POS    = 1 << (' ' as u32 - ' ' as u32);
        const MARK_POS   = 1 << ('+' as u32 - ' ' as u32);
        const GROUPED    = 1 << ('\'' as u32 - ' ' as u32);
    }
}

// 参数联合体 (模块私有)
union Arg {
    i: u64,
    f: f64,    // long double 需要特殊处理
    p: *mut core::ffi::c_void,
}

// 状态机状态 (模块私有)
enum State {
    Bare, Llpre, Hpre, Hhpre, Biglpre,
    Ztpre, Jpre, Stop,
    Ptr, Int, Uint, Ullong, Long, Ulong,
    Short, Ushort, Char, Uchar, Llong,
    Sizet, Imax, Umax, Pdiff, Uiptr,
    Dbl, Ldbl, Noarg, Maxstate,
}
```

---

### 前置/后置条件

**[Pre-condition]:**
- `f` 指向有效的 `FILE` 对象
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- `ap` 由 `va_start` 正确初始化

**[Post-condition]:**
- Case 1 成功：返回写入的宽字符总数（不含 `L'\0'`）
- Case 2 格式错误：返回 `-1`，`errno = EINVAL`
- Case 3 输出溢出：返回 `-1`，`errno = EOVERFLOW`
- Case 4 写入错误：返回 `-1`
- `f->flags` 中的 `F_ERR` 标志在调用前后保持不变

**[Error Behavior]:**
- 格式错误时 `errno = EINVAL`
- 溢出时 `errno = EOVERFLOW`
- 流错误（`ferror(f)`）时返回 `-1`

---

### 不变量

**[Invariant]:**
- `f->flags` 中的 `F_ERR` 位在函数调用前后保持不变
- `va_copy` 确保原始 `va_list` 不被消耗
- 流方向被设置为宽字符模式

---

### 意图

向 FILE 流 `f` 写入宽字符格式化输出。是 `fwprintf` 的 `va_list` 版本，宽字符 printf 家族的核心入口。实现结构完全对称于 `vfprintf`。

核心策略：对于数值类型（`%d`、`%f` 等），构建窄字符 `charfmt` 格式串，委托给 `fprintf`（窄字符引擎）处理实际数字格式化。对于宽字符特有类型（`%C`、`%S`），自行处理。两阶段处理：
- Phase 1（`f == NULL`）：解析格式字符串，提取 `$` 位置参数类型信息
- Phase 2（`f != NULL`）：执行实际格式化输出

---

### 系统算法

```
vfwprintf(f, fmt, ap):
  1. va_copy(ap2, ap)
  // Phase 1: 仅解析格式串，提取位置参数信息
  2. if wprintf_core(NULL, fmt, &ap2, nl_arg, nl_type) < 0:
       va_end(ap2); return -1
  3. FLOCK(f)
  4. fwide(f, 1)
  5. 保存并清除 f->flags 中的 F_ERR 位
  6. ret = wprintf_core(f, fmt, &ap2, nl_arg, nl_type)  // Phase 2: 实际输出
  7. if ferror(f): ret = -1
  8. 恢复 F_ERR 标志
  9. FUNLOCK(f)
  10. va_end(ap2)
  return ret
```

`wprintf_core` 内部：
- 遍历格式字符串，逐段处理字面量文本和 `%` 格式说明符
- 对 `%c`/`%C`：输出单个宽字符
- 对 `%S`：直接输出宽字符串
- 对 `%s`：通过 `mbtowc` 将窄字符串逐字符转换为宽字符后输出
- 对 `%m`：使用 `strerror(errno)` 的错误信息
- 对其他数值类型：构建 `charfmt` 委托给 `fprintf`

时间复杂度 O(n)，n 为输出宽字符总数。

---

## 依赖图

```
vfwprintf (Public)
  ├── wprintf_core (module-private) — 宽字符格式化核心引擎
  │     ├── pop_arg (module-private) — 从 va_list 提取参数
  │     ├── out (module-private) — 向 FILE 输出宽字符
  │     │     └── fputwc (see fputwc.c)
  │     ├── pad (module-private) — 输出填充
  │     │     └── fprintf (see fprintf.c)
  │     ├── getint (module-private) — 解析宽字符格式串中整数
  │     │     └── iswdigit (来自 <wctype.h>)
  │     ├── strerror (来自 <string.h>) — %m 错误信息
  │     ├── mbtowc (来自 <wchar.h>) — 字符串转换
  │     ├── wcsnlen (来自 <wchar.h>) — 宽字符串安全长度
  │     ├── snprintf (来自 <stdio.h>) — 构建 charfmt
  │     ├── fprintf (来自 <stdio.h>) — 委托窄字符格式化
  │     └── btowc (来自 <wchar.h>) — 单字节到宽字符
  ├── fwide (see fwide.c) — 设置流方向
  ├── FLOCK / FUNLOCK (来自 stdio_impl.h)
  ├── ferror (来自 <stdio.h>)
  └── va_copy (<stdarg.h>)
```

---

## [RELY]

- `fputwc` — 宽字符输出 (见 `fputwc.md`)
- `fprintf` — 窄字符格式化输出 (见 `fprintf.md`)
- `fwide` — 流方向设置 (见 `fwide.md`)
- `strerror` — 错误信息 (`<string.h>`)
- `mbtowc` / `wcsnlen` / `btowc` — 转换函数 (`<wchar.h>`)
- `iswdigit` — 宽字符分类 (`<wctype.h>`)
- `FLOCK` / `FUNLOCK` — 流锁定宏 (来自 `stdio_impl.h`)
- `ferror` — 检查流错误状态 (`<stdio.h>`)
- `va_copy` / `va_end` — C99 可变参数宏 (`<stdarg.h>`)
- `EINVAL` / `EOVERFLOW` — 错误码 (`<errno.h>`)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn vfwprintf(f: *mut FILE, fmt: *const c_uint, ap: va_list) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为符合 C99 vfwprintf 语义。
