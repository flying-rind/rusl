# fgetws 函数规约

## 复杂度分级: Level 1

> musl libc 宽字符行读取实现。从 FILE 流中读取一行宽字符串。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};
use crate::internal::FILE;

// fgetws — User, 标准库函数
extern "C" fn fgetws(s: *mut c_uint /* wchar_t */, n: c_int, f: *mut FILE) -> *mut c_uint;

// fgetws_unlocked — User (weak_alias -> fgetws)
extern "C" fn fgetws_unlocked(s: *mut c_uint, n: c_int, f: *mut FILE) -> *mut c_uint;
```

[Visibility]: `fgetws` 是 `<wchar.h>` 标准库函数，对外导出。`fgetws_unlocked` 是 POSIX 免锁扩展，通过弱别名对外导出（在 musl 中与 `fgetws` 指向同一实现）。

Rust 侧实现策略：
- `fgetws` 内部分为 `FLOCK` 保护的循环，逐宽字符通过 `__fgetwc_unlocked` 读取
- 循环逻辑使用安全的 Rust 控制流（for 循环、break）
- 返回判断使用 `Option` 语义（未读取到任何字符返回 NULL）

---

### 前置/后置条件

**[Pre-condition]:**
- `s`: 非空缓冲区指针，至少有 `n` 个 `wchar_t` 的存储空间
- `n`: 缓冲区大小（宽字符数），`n > 0`
- `f`: 非空 FILE 指针，指向已打开的读模式流
- 若 `n == 1`：不读取任何字符，写入 `L'\0'` 后直接返回 `s`

**[Post-condition]:**
- **Case 1 成功读取（包括读到换行符）**
  - 返回 `s`
  - `s` 包含读取的宽字符并以 `L'\0'` 终止
  - 若读到换行符，换行符包含在结果中

- **Case 2 到达文件末尾但未读取任何字符**
  - 返回 `NULL`
  - `s` 内容不变

- **Case 3 读取过程中发生错误**
  - 返回 `NULL`
  - `ferror(f)` 返回非零

- **Case 4 读取过程中到达 EOF（但已读取了一些字符）**
  - 返回 `s`
  - `s` 包含已读取的字符并以 `L'\0'` 终止

**[Error Behavior]:**
- 发生错误时 `ferror(f)` 返回非零

---

### 不变量

**[Invariant]:**
- 始终以 `L'\0'` 终止 `s`（即使没有读取任何字符）
- 函数持有 `FLOCK(f)` 期间逐字符读取

---

### 意图

从 FILE 流 `f` 中读取最多 `n-1` 个宽字符存入 `s`，遇到换行符 `L'\n'` 或 EOF 时停止。读取成功后以 `L'\0'` 终止字符串。

---

### 系统算法

```
fgetws(s, n, f):
  p = s
  if --n == 0:     // n == 1 的特殊情况
    return s       // 空字符串

  FLOCK(f)

  for ; n > 0; n--:
    c = __fgetwc_unlocked(f)
    if c == WEOF: break
    *p++ = c
    if c == '\n': break

  *p = 0
  if ferror(f): p = s

  FUNLOCK(f)

  return (p == s) ? NULL : s
```

时间复杂度 O(n)，n 为读取的宽字符数。

---

## 依赖图

```
fgetws (Public)
  ├── __fgetwc_unlocked (see fgetwc.c)
  ├── ferror (来自 <stdio.h>)
  └── FLOCK / FUNLOCK (来自 stdio_impl.h)

fgetws_unlocked (weak_alias)
  └── fgetws
```

---

## [RELY]

- `__fgetwc_unlocked` — 无锁宽字符读取 (见 `fgetwc.md`)
- `ferror` — 检查流错误状态 (`<stdio.h>`)
- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏 (来自 `stdio_impl.h`)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn fgetws(s: *mut c_uint, n: c_int, f: *mut FILE) -> *mut c_uint;`
  `extern "C" fn fgetws_unlocked(s: *mut c_uint, n: c_int, f: *mut FILE) -> *mut c_uint;`

本模块保证对外提供上述 ABI 兼容的函数符号。`fgetws_unlocked` 为 `fgetws` 的弱别名，行为完全一致。
