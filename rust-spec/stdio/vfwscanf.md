# vfwscanf 函数规约

## 复杂度分级: Level 3

> musl libc 宽字符格式化输入核心引擎。实现 `vfwscanf` 函数及所有内部辅助函数。与 `vfscanf.c` 的结构高度对称，区别在于格式字符串和终端字符处理均为宽字符。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};
use crate::internal::FILE;

// vfwscanf — User, 标准库函数
extern "C" fn vfwscanf(
    f: *mut FILE,
    fmt: *const c_uint, /* const wchar_t */
    ap: va_list
) -> c_int;

// __isoc99_vfwscanf — Internal (weak_alias -> vfwscanf)
extern "C" fn __isoc99_vfwscanf(
    f: *mut FILE,
    fmt: *const c_uint,
    ap: va_list
) -> c_int;
```

[Visibility]: `vfwscanf` 是 `<stdarg.h>` / `<wchar.h>` 标准库函数，对外导出。`__isoc99_vfwscanf` 为 Internal 符号（C99 兼容别名），通过弱别名对外导出。

Rust 侧实现策略：
- `vfwscanf` 函数体保持 ABI 兼容
- 内部辅助函数（`store_int`、`arg_n`、`in_set`）均为模块私有函数
- 长度修饰符编码可用 Rust 的 `enum` 表示
- 内联优化的 `getwc`/`ungetwc` 宏可用内联函数重写
- `%s`/`%c`/`%[` 直接操作宽字符匹配
- `%d`/`%f` 等数值类型委托给 `fscanf`
- `%m` 动态分配使用安全的 `Vec`/`Box` 或其等价物
- 扫描集使用 Rust 的迭代器和闭包

---

### 内部宏定义（模块私有）

```rust
// 长度修饰符编码 (模块私有)
#[repr(i32)]
enum SizeMod {
    Hh   = -2,
    H    = -1,
    Def  = 0,
    L    = 1,
    Ll   = 3,
    Ldbl = 2,  // SIZE_L
}
```

---

### 前置/后置条件

**[Pre-condition]:**
- `f` 指向有效的 `FILE` 对象
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- `ap` 由 `va_start` 正确初始化

**[Post-condition]:**
- Case 1 成功：返回成功匹配并赋值的输入项数（不含 `%n` 和赋值抑制项）
- Case 2 输入失败（首个转换前到达 EOF）：返回 `EOF`
- Case 3 格式错误：返回已成功匹配的项数
- Case 4 匹配失败：返回匹配失败前的成功赋值项数
- Case 5 动态分配失败（`%m` 的 `malloc`/`realloc`）：返回当前已成功匹配的项数

**[Error Behavior]:**
- 输入失败时返回 `EOF`
- 匹配失败时返回已赋值项数（可能为 0）

---

### 不变量

**[Invariant]:**
- 流 `f` 在函数开始时获取锁，返回前释放锁
- `pos` 跟踪已读取的字符总数（用于 `%n`）
- `%m` 分配的内存失败时被释放
- 流方向被设置为宽字符模式
- 对整数/浮点/指针使用窄字符 `fscanf` 委托，仅对 `%s`、`%c`、`%[` 直接进行宽字符处理

---

### 意图

从 FILE 流 `f` 读取宽字符格式化输入。是 `fwscanf` 的 `va_list` 版本，宽字符 scanf 家族的核心入口。核心策略：
- 跳过空白字符：使用 `iswspace` 判断宽字符空白
- `%c` / `%s` / `%[`：直接操作宽字符进行匹配
- `%d` / `%f` 等数值类型：使用 `snprintf` 构建窄字符格式串，委托 `fscanf` 处理数字扫描
- `%S` / `%C`：视为 `%ls` / `%lc`（标准行为）
- `%m`：动态分配内存以接收输入

---

### 系统算法

```
vfwscanf(f, fmt, ap):
  FLOCK(f)
  fwide(f, 1)

  for p in fmt:
    alloc = false

    // 跳过宽字符空白
    if iswspace(*p):
      while iswspace(p[1]): p++
      while iswspace(c = getwc(f)): pos++
      ungetwc(c, f)
      continue

    // 字面量匹配（非%或%%）
    if *p != '%' or p[1] == '%':
      if *p == '%': p++; skip spaces
      else: c = getwc(f)
      if c != *p: match_fail or input_fail
      continue

    // 解析 % 格式说明符
    解析赋值抑制、位置参数、字段宽度、动态分配、长度修饰符
    // %S/%C 转换为宽字符版本

    // 跳过输入空白（%c 和 %[ 不跳过）

    switch type:
      '%n': store_int(dest, size, pos)
      '%s'/'%c'/'%[': 直接宽字符匹配
      '%d'/'%i'/'%f' 等: 委托 fscanf
      default: goto fmt_fail

  FUNLOCK(f)
  return matches
```

时间复杂度 O(n)，n 为输入宽字符总数。

---

## 依赖图

```
vfwscanf (Public)
  ├── store_int (module-private) — 按长度修饰符存储整数
  ├── arg_n (module-private) — 按位置参数索引提取参数
  ├── in_set (module-private) — 宽字符扫描集成员判断
  ├── getwc / ungetwc (来自 <wchar.h>) — 宽字符 I/O（内联优化）
  ├── iswspace / iswdigit (来自 <wctype.h>)
  ├── wctomb (来自 <wchar.h>)
  ├── snprintf / fscanf (来自 <stdio.h>) — 委托窄字符形态
  ├── malloc / realloc / free (来自 <stdlib.h>) — %m 动态分配
  ├── fwide (see fwide.c) — 设置流方向
  └── FLOCK / FUNLOCK (来自 stdio_impl.h)

__isoc99_vfwscanf (weak_alias)
  └── vfwscanf
```

---

## [RELY]

- `getwc` / `ungetwc` — 宽字符 I/O (`<wchar.h>`)
- `iswspace` / `iswdigit` — 宽字符分类 (`<wctype.h>`)
- `wctomb` — 宽字符到多字节转换 (`<wchar.h>`)
- `snprintf` / `fscanf` — 窄字符委托 (`<stdio.h>`)
- `malloc` / `realloc` / `free` — 动态内存 (`<stdlib.h>`)
- `fwide` — 流方向设置 (见 `fwide.md`)
- `FLOCK` / `FUNLOCK` — 流锁定宏 (来自 `stdio_impl.h`)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn vfwscanf(f: *mut FILE, fmt: *const c_uint, ap: va_list) -> c_int;`
  `extern "C" fn __isoc99_vfwscanf(f: *mut FILE, fmt: *const c_uint, ap: va_list) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号。`__isoc99_vfwscanf` 为 `vfwscanf` 的弱别名，行为完全一致。
