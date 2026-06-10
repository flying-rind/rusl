# fgetln 函数规约

## 复杂度分级: Level 1

> musl libc GNU 扩展行读取实现。`fgetln` 返回指向 FILE 流中一行数据的指针，避免拷贝，提供零分配的行读取方式。

---

## 函数接口

```rust
use core::ffi::{c_char, c_uint};

// size_t 使用 usize
// FILE 为 opaque 类型，定义于 rusl-internal 模块

extern "C" fn fgetln(f: *mut FILE, plen: *mut usize) -> *mut c_char;
```

[Visibility]: User — GNU 扩展（`_GNU_SOURCE` 下可见），非 POSIX 标准函数，不保证可移植。必须保持 ABI 兼容：`extern "C"` 导出，参数类型布局与原 C 接口一致。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: 非空 `*mut FILE` 指针，指向已打开的读模式流。
- `plen`: 非空指针，用于接收行长（包括换行符）。

**[Post-condition]:**
- **Case 1 数据已在缓冲区中包含完整行**
  - `*plen` 设置为该行长度（包括 `\n`）。
  - 返回指向 `f->rpos` 原始位置的指针（在 FILE 缓冲区内部）。
  - FILE 流位置前进到换行符之后。
  - **注意**: 返回的指针指向 FILE 内部缓冲区，下次 IO 操作可能使其失效。

- **Case 2 数据不在缓冲区中（或不够完整行），已到达 EOF**
  - 若 `getline` 返回 `> 0`：`*plen` 设置为行长，返回 `f->getln_buf`。
  - 若 `getline` 返回 `<= 0`：返回 `null_mut()`，`*plen` 可能未被修改。

- **Case 3 流无法读取或出错**
  - 返回 `null_mut()`。

**[Error Behavior]:**
- 出错或 EOF 时返回 `null_mut()`。`*plen` 未被修改。
- 返回指针的有效期受限于下次对同一 FILE 流的 IO 操作。

---

### 不变量

**[Invariant]:**
- FILE 锁在整个执行期间被持有。
- 返回指针的有效期受限于下次对 `f` 的 IO 操作。
- 若返回非空指针，`*plen` 必定被正确设置。

---

### 意图

从 FILE 流 `f` 中返回指向下一行数据的指针（包括换行符），并通过 `*plen` 返回该行的长度。相比 `fgets` 和 `getline`：
- **零拷贝**: 若数据已在 FILE 读缓冲区中，直接返回指向缓冲区内部的指针，无需拷贝。
- **零分配**: 仅当缓冲区中数据不足以找到完整行时，才通过 `getline` 动态分配内存到 `f->getln_buf`。

Rust 侧实现：
- 使用 `unsafe` 块访问 `*mut FILE` 的内部字段（`rpos`、`rend`、`getln_buf` 等）。
- `ungetc(getc_unlocked(f), f)` 预读技巧：先用 `getc_unlocked` 确保 FILE 底层缓冲区被填充，再用 `ungetc` 推回，保证 `f->rend` 指向有效数据。
- 在 Rust 中通过指针运算和 `memchr` 实现零拷贝行搜索。

### 系统算法

```
fgetln(f: *mut FILE, plen: *mut usize) -> *mut c_char:
  ret: *mut c_char = null_mut()

  FLOCK(f)                                  // 获取 FILE 锁

  // 步骤 1: 预读一个字符后将其推回
  // 目的: 强制将数据加载到 f->rend 缓冲区
  ungetc(getc_unlocked(f), f)

  // 步骤 2: 尝试从现有缓冲区中查找完整行
  if f->rend != null && (z = memchr(f->rpos, '\n', f->rend - f->rpos)):
    // 缓冲区中找到了 '\n'，返回指向缓冲区的零拷贝指针
    ret = f->rpos as *mut c_char
    *plen = (z + 1) - ret                   // 计算长度(含 '\n')
    f->rpos = z + 1                         // 前进读指针
  else if (l = getline(&f->getln_buf, &(0usize), f)) > 0:
    // 缓冲区中未找到完整行，使用 getline 动态分配
    *plen = l as usize
    ret = f->getln_buf                      // 返回动态分配的缓冲区

  FUNLOCK(f)
  return ret
```

时间复杂度 O(n)，n 为当前缓冲区中的数据量。

---

## 依赖图

```
fgetln (Public)
  ├── FLOCK / FUNLOCK (锁宏, 来自 stdio_impl 模块)
  ├── getc_unlocked (内部宏/内联函数)
  │     └── __uflow (see __uflow spec)
  ├── ungetc (from <stdio.h>)
  ├── memchr (from <string.h>)
  ├── getline (see getline spec)
  │     └── getdelim (see getdelim spec)
  └── f->getln_buf (FILE 内部缓冲区字段)
```

---

## [RELY]

- `FLOCK(f)` / `FUNLOCK(f)` — 条件加锁/解锁宏。
- `getc_unlocked(f)` — 无锁字符读取函数。
- `ungetc(int, *mut FILE)` — 字符推回。
- `memchr` — 内存搜索换行符（`core::slice::memchr` 或复用已有实现）。
- `getline(*mut *mut c_char, *mut usize, *mut FILE)` — 动态分配行读取。
- `f->getln_buf` — FILE 内部行缓冲区字段。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn fgetln(f: *mut FILE, plen: *mut usize) -> *mut c_char;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为符合 GNU `fgetln` 扩展语义。
