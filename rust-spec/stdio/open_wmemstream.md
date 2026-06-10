# open_wmemstream 函数规约

## 复杂度分级: Level 2

> musl libc 宽字符动态内存流打开函数实现。创建一个自动增长的只写流，允许程序动态构建宽字符内存缓冲区，并通过输出参数获取最终缓冲区指针和大小。

---

## 函数接口

```rust
use core::ffi::c_int;
use crate::internal::FILE;

extern "C" fn open_wmemstream(
    bufp: *mut *mut c_int, // wchar_t **
    sizep: *mut usize,     // size_t *
) -> *mut FILE;
```

[Visibility]: `open_wmemstream` 是 `<wchar.h>` POSIX.1-2008 标准接口，对外导出。

Rust 侧实现策略：
- 内部 cookie 和 wms_FILE 结构体为模块私有
- `wms_write`、`wms_seek`、`wms_close` 为模块私有回调函数
- 使用 `malloc`/`realloc`/`free` 管理动态内存（Rust 中对应 `alloc` crate 的分配器）
- `mbstate_t` 状态可用 Rust 的类型安全封装
- 缓冲区扩展逻辑可用安全的 `Vec`-like 抽象
- 通过 `__ofl_add` 注册到全局打开文件链表

---

### 内部类型定义（模块私有）

```rust
// cookie — 宽字符动态内存流的内部状态控制块 (模块私有)
struct Cookie {
    bufp: *mut *mut u32,    // wchar_t **, 指向调用者缓冲区指针的指针
    sizep: *mut usize,      // size_t *, 指向调用者大小变量
    pos: usize,             // 当前写入位置（宽字符偏移）
    buf: *mut u32,          // wchar_t *, 内部分配的宽字符缓冲区
    len: usize,             // 当前有效数据长度（宽字符数）
    space: usize,           // 缓冲区已分配总容量（宽字符数）
    mbs: MbState,           // 多字节到宽字符的转换状态
}

// wms_FILE — 宽字符动态内存流 FILE 对象 (模块私有)
struct WmsFile {
    f: FILE,
    c: Cookie,
    buf: [u8; 1],  // 1 字节的 FILE 缓冲区
}

// 回调函数 (模块私有)
fn wms_write(f: *mut FILE, buf: *const u8, len: usize) -> usize;
fn wms_seek(f: *mut FILE, off: i64 /* off_t */, whence: c_int) -> i64;
fn wms_close(f: *mut FILE) -> c_int;
```

---

### 前置/后置条件

**[Pre-condition]:**
- `bufp`: 指向 `wchar_t*` 的指针（非 `NULL`），将在此处返回最终宽字符缓冲区指针
- `sizep`: 指向 `size_t` 的指针（非 `NULL`），将在此处返回宽字符数（不包含 `L'\0'` 终止符）

**[Post-condition]:**
- **Case 1: 成功** — 返回新创建的 `FILE*` 对象
  - 流已设置为只写模式（`F_NORD` 标志）
  - `fd` 设为 `-1`
  - 初始宽字符缓冲区大小为 `sizeof(wchar_t)`（1 个宽字符 = `*buf = 0`）
  - `*sizep = 0`，`*bufp` 指向初始宽字符缓冲区
  - `fwide` 被调用设置为宽字符模式
  - 自定义回调：`wms_write`、`wms_seek`、`wms_close`
  - 若未启用线程化（`!libc.threaded`），`f->lock = -1`
  - 通过 `__ofl_add` 注册到全局打开文件链表
- **Case 2: 失败** — 返回 `NULL`
  - 若 `malloc` 分配 `f` 或初始 `buf` 失败

**[Error Behavior]:**
- 分配失败时返回 `NULL`
- `wms_seek` 非法参数：`errno = EINVAL`，返回 `-1`
- `wms_write` realloc 失败或多字节转换错误：返回 `0`
- `wms_close` 始终成功返回 `0`

---

### 不变量

**[Invariant]:**
- `fd` 始终为 `-1`
- 流是只写的（`F_NORD` 标志）
- `*sizep` 实时反映当前缓冲区宽字符数
- 缓冲区始终以 `L'\0'` 终止
- 关闭后 `*bufp` 归调用者所有，调用者负责 `free`
- 扩展以 `sizeof(wchar_t)` 为单位（`len2*4` 计算新容量，因 `sizeof(wchar_t) == 4`）

---

### 意图

创建一个只写的宽字符动态内存流。写入的宽字符数据被动态分配到内存缓冲区中。调用者通过 `bufp` 和 `sizep` 获取最终的缓冲区指针和大小。当流被 `fclose` 关闭时，缓冲区被终止为有效的宽字符串，并且 `*bufp` 和 `*sizep` 被更新为最终状态。

与 `open_memstream` 的区别：
- 缓冲区存储宽字符（`wchar_t`）而非字节（`char`）
- 使用 `mbsnrtowcs` 进行多字节到宽字符的增量转换
- 缓冲区扩展以 `sizeof(wchar_t)` 为单位

---

### 系统算法

```
open_wmemstream(bufp, sizep):
  1. f = malloc(sizeof(WmsFile))
     if !f: return NULL
  2. buf = malloc(sizeof(wchar_t))
     if !buf: free(f); return NULL
  3. memset(&f->f, 0, sizeof f->f)
  4. memset(&f->c, 0, sizeof f->c)
  5. f->f.cookie = &f->c
  6. 初始化 cookie:
        c.bufp = bufp
        c.sizep = sizep
        c.pos = c.len = c.space = *sizep = 0
        c.buf = *bufp = buf
        *buf = 0
  7. 初始化 FILE:
        f->f.flags = F_NORD
        f->f.fd = -1
        f->f.buf = f->buf
        f->f.buf_size = 0
        f->f.lbf = EOF
        f->f.write = wms_write
        f->f.seek = wms_seek
        f->f.close = wms_close
  8. if !libc.threaded: f->f.lock = -1
  9. fwide(&f->f, 1)
  10. return __ofl_add(&f->f)

wms_write(f, buf, len):
  // 先递归刷出 FILE 自身写缓冲区中的待写数据
  // 检查是否需要扩容 (以宽字符为单位)
  // realloc 扩展缓冲区
  // mbsnrtowcs 增量多字节到宽字符转换
  // 更新 c->pos, c->len, *sizep
  return len

wms_seek(f, off, whence):
  // 计算新位置 base + off
  // 越界检查: off < -base || off > SSIZE_MAX/4 - base
  // 重置 mbs 转换状态
  return c->pos = base + off

wms_close(f):
  return 0
```

---

## 依赖图

```
open_wmemstream (Public)
  ├── wms_FILE (struct) — 内部定义
  ├── cookie (struct) — 内部定义
  ├── wms_seek (module-private) — seek 回调
  ├── wms_write (module-private) — 写入回调
  │     ├── mbsnrtowcs (来自 <wchar.h>)
  │     └── realloc (来自 <stdlib.h>)
  ├── wms_close (module-private) — 关闭回调
  ├── __ofl_add (see ofl_add.c)
  ├── fwide (see fwide.c)
  ├── malloc / free (来自 <stdlib.h>)
  ├── memset (来自 <string.h>)
  └── libc.threaded (来自 libc.h)
```

---

## [RELY]

- `mbsnrtowcs` — 有状态的多字节到宽字符串转换 (`<wchar.h>`)
- `realloc` / `malloc` / `free` — 动态内存分配 (`<stdlib.h>`)
- `memset` — 内存填充 (`<string.h>`)
- `fwide` — 流方向设置 (见 `fwide.md`)
- `__ofl_add` — 注册到全局打开文件链表 (见 `ofl_add.md`)
- `libc.threaded` — 线程化状态标志 (来自 `libc.h`)
- `SSIZE_MAX` — `ssize_t` 最大值 (`<limits.h>`)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn open_wmemstream(bufp: *mut *mut c_int, sizep: *mut usize) -> *mut FILE;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为符合 POSIX.1-2008 open_wmemstream 语义。
