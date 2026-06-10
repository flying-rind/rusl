# fputws 函数规约

## 复杂度分级: Level 1

> musl libc 宽字符串写入实现。将宽字符串转换为多字节序列并批量写入 FILE 流。

---

## 函数接口

```rust
use core::ffi::c_int;
use crate::internal::FILE;

// fputws — User, 标准库函数
extern "C" fn fputws(ws: *const c_int /* const wchar_t */, f: *mut FILE) -> c_int;

// fputws_unlocked — User (weak_alias -> fputws)
extern "C" fn fputws_unlocked(ws: *const c_int, f: *mut FILE) -> c_int;
```

[Visibility]: `fputws` 是 `<wchar.h>` 标准库函数，对外导出。`fputws_unlocked` 是 POSIX 免锁扩展，通过弱别名对外导出。

Rust 侧实现策略：
- 内部 locale 管理使用 RAII 风格的 guard
- `wcsrtombs` 的循环写入可通过安全的迭代器抽象
- 使用 `BUFSIZ` 大小的栈上数组作为转换缓冲区
- 返回 `0` 表示成功，`-1` 表示失败

---

### 前置/后置条件

**[Pre-condition]:**
- `ws`: 指向以 `L'\0'` 结尾的有效宽字符串；可以为 `NULL`（此时行为为无操作，返回 `0`）
- `f`: 非空 FILE 指针，指向已打开的写模式流

**[Post-condition]:**
- **Case 1 成功写入完整字符串**
  - 返回 `0`
  - 所有宽字符已转换为多字节序列并写入流

- **Case 2 写入错误或编码错误**
  - 返回 `-1`
  - `f->flags` 可能设置 `F_ERR`

**[Error Behavior]:**
- 写入错误或 `wcsrtombs` 转换错误时返回 `-1`

---

### 不变量

**[Invariant]:**
- 不写入终止 `L'\0'`
- 使用流的 locale 进行宽字符到多字节的转换
- 调用者的 locale 在返回时恢复

---

### 意图

将宽字符串 `ws` 转换为多字节序列并写入 FILE 流 `f`。使用 `BUFSIZ` 大小的本地缓冲区进行批量转换，通过 `__fwritex` 每次写入一批转换后的字节。

---

### 系统算法

```
fputws(ws, f):
  buf[BUFSIZ]
  l = 0
  ploc = &CURRENT_LOCALE
  loc = *ploc

  FLOCK(f)
  fwide(f, 1)
  *ploc = f->locale

  while ws && (l = wcsrtombs(buf, &ws, sizeof buf, 0)) + 1 > 1:
    if __fwritex(buf, l, f) < l:
      FUNLOCK(f)
      *ploc = loc
      return -1

  FUNLOCK(f)
  *ploc = loc
  return l   // 0 成功; -1 wcsrtombs 转换错误
```

时间复杂度 O(n*m)，n 为宽字符串长度，m 为多字节转换开销。

---

## 依赖图

```
fputws (Public)
  ├── CURRENT_LOCALE (宏, 来自 locale_impl.h)
  ├── fwide (see fwide.c)
  ├── wcsrtombs (来自 <wchar.h>)
  ├── __fwritex (see fwrite.c)
  └── FLOCK / FUNLOCK (来自 stdio_impl.h)

fputws_unlocked (weak_alias)
  └── fputws
```

---

## [RELY]

- `CURRENT_LOCALE` — 当前线程 locale (来自 `locale_impl.h`)
- `fwide` — 流方向设置 (见 `fwide.md`)
- `wcsrtombs` — 宽字符串到多字节字符串转换 (`<wchar.h>`)
- `__fwritex` — 无锁批量写入 (见 `fwrite.md`)
- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏 (来自 `stdio_impl.h`)
- `BUFSIZ` — 默认缓冲区大小宏 (来自 `stdio_impl.h`)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn fputws(ws: *const c_int, f: *mut FILE) -> c_int;`
  `extern "C" fn fputws_unlocked(ws: *const c_int, f: *mut FILE) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号。`fputws_unlocked` 为 `fputws` 的弱别名，行为完全一致。
