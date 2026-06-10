# vswscanf 函数规约

## 复杂度分级: Level 2

> musl libc 宽字符串格式化输入函数（va_list 版本）。创建自定义只读 FILE 流，从用户提供的宽字符串缓冲区读取格式化输入。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};
use crate::internal::FILE;

// vswscanf — User, 标准库函数
extern "C" fn vswscanf(
    s: *const c_uint,    /* const wchar_t */
    fmt: *const c_uint,  /* const wchar_t */
    ap: va_list
) -> c_int;

// __isoc99_vswscanf — Internal (weak_alias -> vswscanf)
extern "C" fn __isoc99_vswscanf(
    s: *const c_uint,
    fmt: *const c_uint,
    ap: va_list
) -> c_int;
```

[Visibility]: `vswscanf` 是 `<stdarg.h>` / `<wchar.h>` 标准库函数，对外导出。`__isoc99_vswscanf` 为 Internal 符号（C99 兼容别名），通过弱别名对外导出。

Rust 侧实现策略：
- `wstring_read` (static) 为模块私有函数，可安全重构
- 自定义 FILE 对象使用免锁模式（`lock = -1`）
- `wcsrtombs` 的惰性批处理转换可用迭代器模式
- cookie 语义明确：存储宽字符串源的当前位置，每次调用返回一个已转换的字节

---

### 内部函数（模块私有）

```rust
// wstring_read — 自定义 FILE 读取回调 (模块私有)
fn wstring_read(f: *mut FILE, buf: *mut u8, len: usize) -> usize;
```

---

### 前置/后置条件

**[Pre-condition]:**
- `s != NULL`，指向以 `L'\0'` 结尾的有效宽字符串
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- `ap` 由 `va_start` 正确初始化

**[Post-condition]:**
- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达字符串末尾）：返回 `EOF`
- `s` 不被修改（只读）

**[Error Behavior]:**
- 输入失败时返回 `EOF`
- 多字节转换错误时 `wstring_read` 返回 `0`（触发 EOF 状态）

---

### 不变量

**[Invariant]:**
- 自定义 FILE 为免锁模式（`lock = -1`）
- 自定义 FILE 使用 `buf_size` 字节的栈上缓冲区作为转换中间件
- `wcsrtombs` 在缓冲区填满或宽字符串耗尽时停止，下一次 `wstring_read` 调用从上次停止位置继续

---

### 意图

从宽字符串 `s` 读取格式化输入。通过创建自定义 FILE 对象并设置 `wstring_read` 为读取回调，将 `vfwscanf` 的输入源重定向到用户提供的宽字符串。

`wstring_read` 采用惰性转换策略：
1. 每次调用通过 `wcsrtombs` 将一批宽字符转换为多字节数据写入 FILE 缓冲区
2. 更新 FILE 读指针（`rpos`/`rend`）
3. 返回一个字节给调用者
4. 下一次调用时继续从停止位置转换下一批

---

### 系统算法

```
wstring_read(f, buf, len):
  src = f->cookie
  if !src: return 0

  k = wcsrtombs(f->buf, &src, f->buf_size, 0)
  if k == (size_t)-1:
    f->rpos = f->rend = 0
    return 0

  f->rpos = f->buf
  f->rend = f->buf + k
  f->cookie = (void *)src

  if !len || !k: return 0

  *buf = *f->rpos++
  return 1

vswscanf(s, fmt, ap):
  buf[256]
  FILE f = {
    .buf = buf,
    .buf_size = sizeof buf,
    .cookie = (void *)s,
    .read = wstring_read,
    .lock = -1,   // 免锁 FILE
  }
  return vfwscanf(&f, fmt, ap)
```

时间复杂度 O(n*m)，n 为输入宽字符数，m 为宽字符到多字节的转换开销。

---

## 依赖图

```
vswscanf (Public)
  ├── wstring_read (module-private) — 读取回调
  │     ├── wcsrtombs (来自 <wchar.h>) — 宽字符串到多字节转换
  │     └── f->buf / f->rpos / f->rend / f->cookie — FILE 缓冲区管理
  └── vfwscanf (see vfwscanf.c) — 格式化引擎

__isoc99_vswscanf (weak_alias)
  └── vswscanf
```

---

## [RELY]

- `vfwscanf` — 宽字符格式化输入核心引擎 (见 `vfwscanf.md`)
- `wcsrtombs` — 宽字符串到多字节字符串转换 (`<wchar.h>`)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn vswscanf(s: *const c_uint, fmt: *const c_uint, ap: va_list) -> c_int;`
  `extern "C" fn __isoc99_vswscanf(s: *const c_uint, fmt: *const c_uint, ap: va_list) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号。`__isoc99_vswscanf` 为 `vswscanf` 的弱别名，行为完全一致。
