# fgetwc 函数规约

## 复杂度分级: Level 2

> musl libc 宽字符单字符读取实现。从 FILE 流中读取一个宽字符，处理多字节到宽字符的转换。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};
use crate::internal::FILE;

// __fgetwc_unlocked_internal — Internal, static 核心引擎
fn __fgetwc_unlocked_internal(f: *mut FILE) -> c_uint; // wint_t

// __fgetwc_unlocked — Internal, hidden 可见性
extern "C" fn __fgetwc_unlocked(f: *mut FILE) -> c_uint;

// fgetwc_unlocked — User (weak_alias -> __fgetwc_unlocked)
extern "C" fn fgetwc_unlocked(f: *mut FILE) -> c_uint;

// getwc_unlocked — User (weak_alias -> __fgetwc_unlocked)
extern "C" fn getwc_unlocked(f: *mut FILE) -> c_uint;

// fgetwc — User, 标准库函数
extern "C" fn fgetwc(f: *mut FILE) -> c_uint;
```

[Visibility]: `fgetwc` 是 `<wchar.h>` 标准库函数，对外导出。`fgetwc_unlocked` 和 `getwc_unlocked` 是 POSIX 免锁扩展，通过弱别名 (`#[no_mangle]` + 函数体复用) 对外导出。`__fgetwc_unlocked` 为 Internal 符号 (`hidden` 可见性)。`__fgetwc_unlocked_internal` 为模块私有函数（原 C 中为 `static`），不导出。

Rust 侧实现策略：
- 内部辅助函数 `__fgetwc_unlocked_internal` 使用安全的 Rust 抽象（避免裸指针操作，使用 `Option`/`Result` 处理错误流）
- `__fgetwc_unlocked` 通过 `#[no_mangle]` 导出为 hidden 符号，保持 ABI 兼容
- `fgetwc_unlocked` 和 `getwc_unlocked` 作为 `__fgetwc_unlocked` 的弱别名，复用同一函数体
- `fgetwc` 通过 `FLOCK`/`FUNLOCK` 宏包装 `__fgetwc_unlocked`

---

### 前置/后置条件

**[Pre-condition]:**
- `fgetwc`: `f` 为非空 FILE 指针，指向已打开的流
- `fgetwc_unlocked` / `getwc_unlocked`: `f` 为非空 FILE 指针；调用者自行负责锁管理
- `__fgetwc_unlocked`: `f` 为非空 FILE 指针；若 `f->mode <= 0`，内部调用 `fwide(f, 1)` 设置宽字符方向
- `__fgetwc_unlocked_internal`: `f` 为非空 FILE 指针，调用者已持有 `f` 的锁；流的 locale 已正确设置

**[Post-condition]:**
- **Case 1 成功转换宽字符**
  - 返回转换后的 `wchar_t` 值（`wint_t` 类型）
  - `f->rpos` 前进已消费的字节数

- **Case 2 到达文件末尾（首字节即 EOF）**
  - 返回 `WEOF`
  - 不设置 `F_ERR` 和 `errno`

- **Case 3 编码错误（非首字节的 EOF 或无效序列）**
  - 返回 `WEOF`
  - `f->flags |= F_ERR`，`errno = EILSEQ`
  - 若有多余字节，调用 `ungetc` 将其推回

**[Error Behavior]:**
- 编码错误时设置 `errno = EILSEQ`
- EOF 不设置 errno

---

### 不变量

**[Invariant]:**
- 宽字符读取始终使用流的 locale 进行多字节转换（`__fgetwc_unlocked` 负责 locale 保存/恢复）
- 调用者的 locale 在 `__fgetwc_unlocked` 返回时恢复
- `fgetwc` 持有 `FLOCK(f)` 期间执行，返回前释放

---

### 意图

提供宽字符单字符读取功能。`fgetwc` 为线程安全的标准接口（带锁），`fgetwc_unlocked` 为免锁版本。内部通过两阶段策略实现多字节到宽字符的转换：
1. 优化路径：若读缓冲区有足够数据，直接用 `mbtowc` 批量转换
2. 逐字节路径：若缓冲区不包含完整多字节字符，逐字节用 `mbrtowc` 增量转换

---

### 系统算法

```
__fgetwc_unlocked_internal(f):
  // Phase 1: 从缓冲区直接转换
  if (f->rpos != f->rend):
    l = mbtowc(&wc, f->rpos, f->rend - f->rpos)
    if (l + 1 >= 1):  // l >= 0 或 l == -2
      f->rpos += l + (l == 0 ? 1 : 0)
      return wc

  // Phase 2: 逐字节读取并转换
  mbstate_t st = {0}
  first = true
  loop:
    b = c = getc_unlocked(f)
    if c < 0:  // EOF
      if !first:  // 非首字节的 EOF 是编码错误
        f->flags |= F_ERR
        errno = EILSEQ
      return WEOF
    l = mbrtowc(&wc, &b, 1, &st)
    if l == -1:  // 无效序列
      if !first:
        f->flags |= F_ERR
        ungetc(b, f)
      return WEOF
    first = false
    if l != -2: break  // 不是不完整序列
  return wc

__fgetwc_unlocked(f):
  ploc = &CURRENT_LOCALE
  loc = *ploc
  if f->mode <= 0: fwide(f, 1)
  *ploc = f->locale
  wc = __fgetwc_unlocked_internal(f)
  *ploc = loc
  return wc

fgetwc(f):
  FLOCK(f)
  c = __fgetwc_unlocked(f)
  FUNLOCK(f)
  return c
```

时间复杂度 O(n)，n 为多字节字符的字节数。

---

## 依赖图

```
fgetwc (Public)
  └── __fgetwc_unlocked (hidden)
        ├── CURRENT_LOCALE (宏, 来自 locale_impl.h)
        ├── fwide (see fwide.c)
        └── __fgetwc_unlocked_internal (module-private)
              ├── mbtowc (来自 <wchar.h>)
              ├── mbrtowc (来自 <wchar.h>)
              ├── getc_unlocked (来自 stdio_impl.h)
              │     └── __uflow (see __uflow.c)
              └── ungetc (来自 <stdio.h>)

fgetwc_unlocked (weak_alias) ──> __fgetwc_unlocked
getwc_unlocked (weak_alias) ──> __fgetwc_unlocked
```

---

## [RELY]

- `mbtowc` / `mbrtowc` — 多字节到宽字符转换 (来自 `<wchar.h>`)
- `getc_unlocked` — 无锁字节读取 (来自 `stdio_impl.h`)
- `ungetc` — 推回字符 (`<stdio.h>`)
- `fwide` — 流方向设置 (见 `fwide.md`)
- `CURRENT_LOCALE` — 当前线程 locale (来自 `locale_impl.h`)
- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏 (来自 `stdio_impl.h`)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn fgetwc(f: *mut FILE) -> c_uint;`
  `extern "C" fn fgetwc_unlocked(f: *mut FILE) -> c_uint;`
  `extern "C" fn getwc_unlocked(f: *mut FILE) -> c_uint;`
  `extern "C" fn __fgetwc_unlocked(f: *mut FILE) -> c_uint;`

本模块保证对外提供上述 ABI 兼容的函数符号。`fgetwc_unlocked` 和 `getwc_unlocked` 为 `__fgetwc_unlocked` 的弱别名，行为完全一致。
