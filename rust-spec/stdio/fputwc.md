# fputwc 函数规约

## 复杂度分级: Level 2

> musl libc 宽字符单字符写入实现。将一个宽字符转换为多字节序列并写入 FILE 流。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};
use crate::internal::FILE;

// __fputwc_unlocked — Internal, hidden 可见性
extern "C" fn __fputwc_unlocked(c: c_uint /* wchar_t */, f: *mut FILE) -> c_uint; // wint_t

// fputwc_unlocked — User (weak_alias -> __fputwc_unlocked)
extern "C" fn fputwc_unlocked(c: c_uint, f: *mut FILE) -> c_uint;

// putwc_unlocked — User (weak_alias -> __fputwc_unlocked)
extern "C" fn putwc_unlocked(c: c_uint, f: *mut FILE) -> c_uint;

// fputwc — User, 标准库函数
extern "C" fn fputwc(c: c_uint, f: *mut FILE) -> c_uint;
```

[Visibility]: `fputwc` 是 `<wchar.h>` 标准库函数，对外导出。`fputwc_unlocked` 和 `putwc_unlocked` 是 POSIX 免锁扩展，通过弱别名对外导出。`__fputwc_unlocked` 为 Internal 符号 (`hidden` 可见性)。

Rust 侧实现策略：
- `__fputwc_unlocked` 内部按三级写入策略组织：ASCII 快速路径、宽缓冲区路径、回退路径
- 内部 locale 管理使用安全的 Rust 抽象（RAII 风格的 locale guard）
- `isascii` 判断可内联为简单的位运算
- 宽字符写缓冲区管理使用安全的切片操作
- `fputwc_unlocked` 和 `putwc_unlocked` 作为 `__fputwc_unlocked` 的弱别名

---

### 前置/后置条件

**[Pre-condition]:**
- `fputwc`: `c` 为要写入的宽字符（`wchar_t` 类型，有效 Unicode 码点或 `WEOF`）；`f` 为非空 FILE 指针
- `fputwc_unlocked` / `putwc_unlocked`: `c` 同上；`f` 同上；调用者自行负责锁管理
- `__fputwc_unlocked`: `c` 和 `f` 同上；若 `f->mode <= 0`，内部调用 `fwide(f, 1)` 设置宽字符方向

**[Post-condition]:**
- **Case 1 成功写入宽字符**
  - 返回写入的宽字符值 `c`
  - 宽字符已转换为多字节序列并写入流的写缓冲区

- **Case 2 写入错误或编码错误**
  - 返回 `WEOF`
  - `f->flags |= F_ERR`

**[Error Behavior]:**
- 写入错误或编码错误时设置 `F_ERR` 标志

---

### 不变量

**[Invariant]:**
- 宽字符写入始终使用流的 locale 进行多字节转换
- 调用者的 locale 在返回时恢复
- `fputwc` 持有 `FLOCK(f)` 期间执行

---

### 意图

提供宽字符单字符写入功能。采用三级写入策略，按优先级选择最高效的路径：
1. ASCII 优化路径：若 `c` 是 ASCII 字符，直接委托 `putc_unlocked`
2. 宽缓冲区路径：若宽字符缓冲区有足够空间，直接写入并转换
3. 回退路径：通过临时缓冲区转换后批量写入

---

### 系统算法

```
__fputwc_unlocked(c, f):
  ploc = &CURRENT_LOCALE
  loc = *ploc
  if f->mode <= 0: fwide(f, 1)
  *ploc = f->locale

  if isascii(c):                       // 路径 1: ASCII 快速路径
    c = putc_unlocked(c, f)
  else if f->wpos + MB_LEN_MAX < f->wend: // 路径 2: 宽字符缓冲区可用
    l = wctomb(f->wpos, c)
    if l < 0: c = WEOF
    else: f->wpos += l
  else:                                // 路径 3: 回退到临时缓冲区
    l = wctomb(mbc, c)
    if l < 0 || __fwritex(mbc, l, f) < l:
      c = WEOF

  if c == WEOF: f->flags |= F_ERR
  *ploc = loc
  return c

fputwc(c, f):
  FLOCK(f)
  c = __fputwc_unlocked(c, f)
  FUNLOCK(f)
  return c
```

时间复杂度 O(n)，n 为多字节字符的字节数。

---

## 依赖图

```
fputwc (Public)
  └── __fputwc_unlocked (hidden)
        ├── CURRENT_LOCALE (宏, 来自 locale_impl.h)
        ├── fwide (see fwide.c)
        ├── isascii (来自 <ctype.h>)
        ├── putc_unlocked (来自 stdio_impl.h)
        │     └── __overflow (see __overflow.c)
        ├── wctomb (来自 <wchar.h>)
        └── __fwritex (来自 stdio_impl.h)

fputwc_unlocked (weak_alias) ──> __fputwc_unlocked
putwc_unlocked (weak_alias) ──> __fputwc_unlocked
```

---

## [RELY]

- `CURRENT_LOCALE` — 当前线程 locale (来自 `locale_impl.h`)
- `fwide` — 流方向设置 (见 `fwide.md`)
- `isascii` — ASCII 字符判断 (`<ctype.h>`)
- `putc_unlocked` — 无锁字节写入 (来自 `stdio_impl.h`)
- `wctomb` — 宽字符到多字节转换 (`<wchar.h>`)
- `__fwritex` — 无锁缓冲批量写入 (见 `fwrite.md`)
- `MB_LEN_MAX` — 多字节字符最大长度 (`<limits.h>`)
- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏 (来自 `stdio_impl.h`)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn fputwc(c: c_uint, f: *mut FILE) -> c_uint;`
  `extern "C" fn fputwc_unlocked(c: c_uint, f: *mut FILE) -> c_uint;`
  `extern "C" fn putwc_unlocked(c: c_uint, f: *mut FILE) -> c_uint;`
  `extern "C" fn __fputwc_unlocked(c: c_uint, f: *mut FILE) -> c_uint;`

本模块保证对外提供上述 ABI 兼容的函数符号。`fputwc_unlocked` 和 `putwc_unlocked` 为 `__fputwc_unlocked` 的弱别名，行为完全一致。
