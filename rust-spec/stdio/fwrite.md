# fwrite 函数规约

## 复杂度分级: Level 1

> musl libc 标准 IO 二进制块写入实现。提供内部辅助函数 `__fwritex`（无锁的底层写入引擎）和公开的 `fwrite` / `fwrite_unlocked` 接口。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uchar, c_void};

// FILE 为 opaque 类型，定义于 rusl-internal 模块

// __fwritex: Internal — 无锁底层写入引擎
extern "C" fn __fwritex(s: *const c_uchar, l: usize, f: *mut FILE) -> usize;

// fwrite: 加锁版本
extern "C" fn fwrite(src: *const c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;

// fwrite_unlocked: 弱别名，与 fwrite 共享同一实现
// weak_alias: fwrite_unlocked 是 fwrite 的弱别名
extern "C" fn fwrite_unlocked(src: *const c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;
```

[Visibility]:
- `__fwritex`: Internal — 模块内部可见（被 `fwrite`、`fputs` 等内部调用），在 musl 中为 `hidden` 属性。但 Rust 中若其他 crate 需要调用（如 `fputs`），可设为 `pub(crate)` 可见性。需 `extern "C"` 导出以保持 ABI 兼容。
- `fwrite`: User — `<stdio.h>` 标准库函数，用户程序可直接调用。
- `fwrite_unlocked`: User — POSIX 免锁 `fwrite`，在 musl 中与 `fwrite` 共享同一实现。

所有符号均必须保持 ABI 兼容：`extern "C"` 导出，参数类型布局与原 C 接口一致。

---

## 函数规约

### 1. __fwritex

#### 前置/后置条件

**[Pre-condition]:**
- `s`: 非空指针，指向至少 `l` 字节的有效数据。
- `l`: 要写入的字节数。
- `f`: 非空 `*mut FILE` 指针。
- 调用者已持有 `f->lock`（或 FILE 为免锁模式）。

**[Post-condition]:**
- **Case 1 完全成功写入**
  - 返回 `l`（全部字节已写入或缓冲）。
  - 数据已复制到 FILE 写缓冲区或通过底层 I/O 写入。

- **Case 2 行缓冲模式 - 部分刷出**
  - 返回 `l + i`，其中 `i` 是通过 `f->write` 刷出的字节数。
  - 剩余数据已缓冲在 `f->wpos`。

- **Case 3 写入失败**
  - 返回 0（`__towrite` 失败时）或 `< l`（`f->write` 部分写入）。
  - `f->flags` 的 `F_ERR` 可能被设置。

**[Error Behavior]:**
- `__towrite` 失败返回 0。`f->write` 部分写入返回 `< l`。

---

#### 系统算法

```
__fwritex(s: *const c_uchar, l: usize, f: *mut FILE) -> usize:
  i: usize = 0

  // 步骤 1: 确保有写缓冲区
  if (*f).wend.is_null() && !__towrite(f): return 0

  // 步骤 2: 数据大于缓冲区剩余空间，直接系统调用写入
  if l > (*f).wend as usize - (*f).wpos as usize:
    return ((*f).write)(f, s as *const c_void, l)

  // 步骤 3: 行缓冲模式下检查并刷出换行前缀
  if (*f).lbf >= 0:
    i = l
    while i > 0 && s[i-1] != '\n' as u8: i -= 1  // 从末尾找换行
    if i > 0:                                      // 找到换行
      n = ((*f).write)(f, s as *const c_void, i)
      if n < i: return n                           // 写入失败
      s = s.add(i)
      l = l - i                                    // 剩余部分将缓冲

  // 步骤 4: 拷贝剩余数据到缓冲区
  ptr::copy_nonoverlapping(s, (*f).wpos, l)
  (*f).wpos = (*f).wpos.add(l)
  return l + i
```

时间复杂度 O(l)，l 为要写入的字节数（行缓冲模式下主要为搜索换行符的时间）。

---

### 2. fwrite

#### 前置/后置条件

**[Pre-condition]:**
- `src`: 非空指针（除非 `size` 或 `nmemb` 为 0），指向至少 `size * nmemb` 字节的有效数据。
- `size`: 每个元素的大小（字节），可为 0。
- `nmemb`: 要写入的元素数量，可为 0。
- `f`: 非空 `*mut FILE` 指针，指向已打开的流（写模式或可写模式）。

**[Post-condition]:**
- **Case 1 完全成功写入全部元素**
  - 返回 `nmemb`。
  - 所有数据已写入或缓冲。

- **Case 2 写入部分元素后出错**
  - 返回 `< nmemb`（实际完整写入的元素数，即 `(已写入字节数) / size`）。

- **Case 3 `size` 为 0**
  - 返回 0，不执行任何写操作。

**[Error Behavior]:**
- 写入失败时返回 `< nmemb`。

---

#### 系统算法

```
fwrite(src: *const c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize:
  if size == 0: return 0
  l = size * nmemb                           // 总字节数

  FLOCK(f)                                   // 获取 FILE 锁
  k = __fwritex(src as *const c_uchar, l, f) // 委托无锁写入引擎
  FUNLOCK(f)                                 // 释放锁

  return if k == l { nmemb } else { k / size }
```

时间复杂度 O(l)，l 为实际写入的字节数。

---

### 3. fwrite_unlocked

前置/后置条件及行为：完全等同于 `fwrite`。在 Rust 侧通过复制函数体实现，或使用 linker 脚本实现真正的弱符号别名。

---

## 依赖图

```
fwrite (Public)
  ├── FLOCK / FUNLOCK (锁宏, 来自 stdio_impl 模块)
  └── __fwritex (Internal)

__fwritex (Internal, hidden)
  ├── __towrite (see __towrite spec)
  ├── f->write (FILE 函数指针, 通常指向 __stdio_write)
  └── ptr::copy_nonoverlapping (替代 memcpy)
```

---

## 不变量

**[Invariant]:**
- `f->wpos` 始终指向写缓冲区的下一个可写位置。
- `f->wend` 指向写缓冲区的末尾。
- `fwrite` 中 `f->lock` 在 FLOCK/FUNLOCK 配对期间被正确持有。

---

## [RELY]

- `FLOCK(f)` / `FUNLOCK(f)` — 条件加锁/解锁。
- `__towrite(*mut FILE)` — 确保 FILE 处于写模式。
- `f->write` — FILE 底层写函数指针（通常指向 `__stdio_write`）。
- `ptr::copy_nonoverlapping` — 内存拷贝（替代 C `memcpy`）。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn __fwritex(s: *const c_uchar, l: usize, f: *mut FILE) -> usize;`
  `extern "C" fn fwrite(src: *const c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;`
  `extern "C" fn fwrite_unlocked(src: *const c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;`

本模块保证对外提供上述三个 ABI 兼容的函数符号。`__fwritex` 为内部辅助函数，被 `fwrite` / `fputs` 等调用。`fwrite` 和 `fwrite_unlocked` 行为符合 ISO C 标准 `fwrite` 语义，二者为弱别名关系。
