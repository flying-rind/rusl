# vswprintf 函数规约

## 复杂度分级: Level 2

> musl libc 宽字符串格式化输出函数（va_list 版本）。创建自定义只写 FILE 流，将格式化宽字符输出写入用户提供的宽字符缓冲区。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};
use crate::internal::FILE;

// vswprintf — User, 标准库函数
extern "C" fn vswprintf(
    s: *mut c_uint,      /* wchar_t */
    n: usize,            /* size_t */
    fmt: *const c_uint,  /* const wchar_t */
    ap: va_list
) -> c_int;
```

[Visibility]: `vswprintf` 是 `<stdarg.h>` / `<wchar.h>` 标准库函数，对外导出。

Rust 侧实现策略：
- `sw_write` (static) 为模块私有函数，可安全重构为 Rust 风格的写入回调
- 内部 cookie 状态管理可用 Rust 的结构体封装
- `mbtowc` 的递归刷出模式可用 Rust 的安全迭代器
- 自定义 FILE 对象可复用 `stdio_impl.h` 的 FILE 结构，通过安全抽象访问
- 注意：musl 实现中截断时返回 `-1`（非 C99 标准）

---

### 内部类型定义（模块私有）

```rust
// cookie — 内部状态控制块 (模块私有)
struct Cookie {
    ws: *mut u32,   // wchar_t*, 目标缓冲区当前写入位置
    l: usize,       // 剩余可写入的宽字符数（含 L'\0' 终止符）
}

// sw_write — 自定义 FILE 写入回调 (模块私有)
fn sw_write(f: *mut FILE, s: *const u8, l: usize) -> usize;
```

---

### 前置/后置条件

**[Pre-condition]:**
- `s`: 指向有效宽字符缓冲区的指针（`n > 0` 时）；`n == 0` 时可为 `NULL`
- `n`: 缓冲区大小（宽字符数）
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- `ap` 由 `va_start` 正确初始化

**[Post-condition]:**
- Case 1 成功（输出未截断）：返回写入的宽字符数（不含 `L'\0'`）
- Case 2 截断（`ret >= n`）：返回 `-1`
- Case 3 `n == 0`：返回 `-1`（尚未调用 `vfwprintf` 即返回）
- Case 4 输出错误/格式错误/溢出：返回 `-1`
- `s` 始终被 `L'\0'` 终止（当 `n > 0` 时）

**[Error Behavior]:**
- 截断时返回 `-1`
- 其他错误同 `vfwprintf`

---

### 不变量

**[Invariant]:**
- 缓冲区始终以 `L'\0'` 终止
- 自定义 FILE 为免锁模式（`lock = -1`），因为不涉及多线程
- 自定义 FILE 不进行行缓冲（`lbf = EOF`，始终全缓冲）

---

### 意图

将格式化宽字符串输出到缓冲区 `s`，最多写入 `n` 个宽字符（含终止 `L'\0'`）。通过创建自定义 FILE 对象并设置 `sw_write` 为写入回调，将 `vfwprintf` 的输出重定向到用户提供的宽字符缓冲区。

musl 实现中 `sw_write` 采用递归刷出策略：
1. 先递归处理 FILE 自身写缓冲区中的待写数据
2. 再逐字符通过 `mbtowc` 将多字节数据转换为宽字符
3. 写入目标缓冲区并更新 cookie 状态

---

### 系统算法

```
sw_write(f, s, l):
  c = f->cookie
  l0 = l

  // 先递归刷出 FILE 自身缓冲区中的待写数据
  if s != f->wbase:
    if sw_write(f, f->wbase, f->wpos - f->wbase) == -1: return -1

  // 逐字符将多字节数据转换为宽字符
  while c->l > 0 && l > 0:
    i = mbtowc(c->ws, s, l)
    if i >= 0:
      if i == 0: i = 1
      s += i; l -= i
      c->l--; c->ws++
    else:  // 转换错误
      f->wpos = f->wbase = f->wend = 0
      f->flags |= F_ERR
      return i

  *c->ws = 0
  f->wend = f->buf + f->buf_size
  f->wpos = f->wbase = f->buf
  return l0

vswprintf(s, n, fmt, ap):
  if n == 0: return -1

  buf[256]              // 栈上 FILE 缓冲区
  cookie = { s, n - 1 } // n-1 为终止符预留空间
  FILE f = {
    .lbf = EOF,         // 禁用行缓冲
    .write = sw_write,  // 自定义写入回调
    .lock = -1,         // 免锁 FILE
    .buf = buf,
    .buf_size = sizeof buf,
    .cookie = &c,
  }

  r = vfwprintf(&f, fmt, ap)
  sw_write(&f, 0, 0)    // 确保最终刷出

  return (r >= n) ? -1 : r
```

时间复杂度 O(n*m)，n 为输出宽字符数，m 为多字节转换开销。

---

## 依赖图

```
vswprintf (Public)
  ├── cookie (struct) — 内部状态
  ├── sw_write (module-private) — 写入回调
  │     ├── mbtowc (来自 <wchar.h>) — 多字节到宽字符转换
  │     └── f->wbase / f->wpos / f->wend — FILE 缓冲区管理
  └── vfwprintf (see vfwprintf.c) — 格式化引擎

swprintf (Public)
  └── vswprintf
```

---

## [RELY]

- `vfwprintf` — 宽字符格式化输出核心引擎 (见 `vfwprintf.md`)
- `mbtowc` — 多字节到宽字符转换 (`<wchar.h>`)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn vswprintf(s: *mut c_uint, n: usize, fmt: *const c_uint, ap: va_list) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号。
