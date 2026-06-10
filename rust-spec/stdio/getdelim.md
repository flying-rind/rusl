# getdelim 函数规约

## 复杂度分级: Level 2

> musl libc 带分隔符的动态行读取实现。`getdelim` 从 FILE 流中读取以指定分隔符（或 EOF）结尾的一行，自动扩展或分配输出缓冲区。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};

// FILE 为 opaque 类型，定义于 rusl-internal 模块

// getdelim: 用户可见函数
extern "C" fn getdelim(s: *mut *mut c_char, n: *mut usize, delim: c_int, f: *mut FILE) -> isize;

// __getdelim: 弱别名，与 getdelim 共享同一实现
// weak_alias: __getdelim 是 getdelim 的弱别名
extern "C" fn __getdelim(s: *mut *mut c_char, n: *mut usize, delim: c_int, f: *mut FILE) -> isize;
```

[Visibility]:
- `getdelim`: User — POSIX.1-2008 标准函数，用户程序包含 `<stdio.h>` 即可调用。
- `__getdelim`: Internal — `__` 前缀别名，供 musl 内部直接调用（绕过宏定义）。

两者均必须保持 ABI 兼容：`extern "C"` 导出，参数类型布局与原 C 接口一致。返回值为 `isize`（对应 C 的 `ssize_t`）。

---

### 前置/后置条件

**[Pre-condition]:**
- `s`: 非空指针，指向 `*mut c_char` 变量（该变量可为 `null_mut()` 或指向已有缓冲区）。
- `n`: 非空指针，指向 `usize` 变量（指示当前缓冲区大小，`*s` 为 `null_mut()` 时自动设为 0）。
- `delim`: 分隔符字符（以 `c_int` 传递，内部转为 `unsigned char` 比较）。
- `f`: 非空 `*mut FILE` 指针，指向已打开的读模式流。

**[Post-condition]:**
- **Case 1 成功读取一行（以分隔符结尾）**
  - 返回读取的字符数（**包括**分隔符字符，不包括结尾的 `\0`）。
  - `*s` 指向包含完整行 + `\0` 的动态分配缓冲区。
  - `*n` 更新为当前缓冲区大小。
  - `*s` 中第 `返回值` 字节为分隔符，第 `返回值 + 1` 字节为 `\0`。

- **Case 2 成功读取到文件末尾（无分隔符）**
  - 返回读取的字符数（不包括 `\0`，**不包括**分隔符）。
  - `*s` 指向包含数据 + `\0` 的缓冲区。
  - FILE 流设置 `F_EOF` 标志。

- **Case 3 参数无效**
  - `s` 或 `n` 为 `null_mut()`：返回 -1，`errno = EINVAL`，FILE 设置 `F_ERR`。
  - 此情况下 `*s` 和 `*n` 保持不变。

- **Case 4 内存不足（realloc 失败）**
  - 返回 -1，`errno = ENOMEM`，FILE 设置 `F_ERR`。
  - 在返回前尽可能将已读取数据拷贝到现有缓冲区（截断适配）。
  - `f->rpos` 位置已前进（缓冲区数据已被消费）。

- **Case 5 读取错误（且未读取到任何字符）**
  - 返回 -1。

**[Error Behavior]:**
- 参数无效返回 -1 并设置 `errno = EINVAL`。
- 内存不足返回 -1 并设置 `errno = ENOMEM`。
- 函数尽力在返回前保存已读取的数据。

---

### 不变量

**[Invariant]:**
- `i` 始终等于已存入 `*s` 的字符数（不含 `\0`）。
- `*s` 始终指向至少 `*n` 字节的有效可用缓冲区（`null_mut()` 时除外，但会被自动分配）。
- FILE 锁在整个执行期间被持有（除非早期返回错误）。

---

### 意图

从 FILE 流 `f` 中读取以字符 `delim` 分隔（或 EOF 结尾）的一行数据到动态分配的缓冲区 `*s`。与 `fgets` 相比：
- **自动分配**: 若 `*s` 为 `null_mut()` 或 `*n` 不足以容纳完整行，自动通过 `realloc` 扩展。
- **分隔符灵活**: 可指定任意字符作为行分隔符（不限于 `\n`）。
- **保留分隔符**: 读取内容包含分隔符（除 EOF 结尾外）。

Rust 侧实现：
- 使用 `unsafe` 块操作双重指针和 FILE 内部字段。
- 缓冲区增长策略：首次分配 `m = i + k + 2`；未找到分隔符时使用几何增长 `m += m/2`（减少 realloc 调用次数）。
- 字符推回技巧：当 `getc_unlocked` 读取的字符无法放入输出缓冲区时，通过 `*--f->rpos = c` 推回。
- 内存耗尽时的尽力而为：即使 realloc 失败，也会将现有缓冲区数据拷贝给调用者（截断适配）。
- `__getdelim` 在 Rust 侧通过复制函数体实现，或使用 linker 脚本实现真正的弱符号别名。

### 系统算法

```
getdelim(s: *mut *mut c_char, n: *mut usize, delim: c_int, f: *mut FILE) -> isize:
  i: usize = 0                              // 已读取字符计数

  FLOCK(f)

  // 参数校验
  if n.is_null() || s.is_null():
    (*f).mode |= (*f).mode - 1
    (*f).flags |= F_ERR
    FUNLOCK(f)
    set_errno(EINVAL)
    return -1

  // 初始分配
  if (*s).is_null(): *n = 0

  loop:
    // 步骤 1: 从 FILE 缓冲区搜索分隔符
    if (*f).rpos != (*f).rend:
      z = memchr((*f).rpos, delim as u8, (*f).rend - (*f).rpos)
      k = if z.is_some() { z_offset + 1 } else { (*f).rend - (*f).rpos }
    else:
      z = null_mut(); k = 0

    // 步骤 2: 检验输出缓冲是否足够，不足则扩展
    if i + k >= *n:
      m = i + k + 2
      if z.is_null() && m < SIZE_MAX / 4: m += m / 2  // 几何增长
      tmp = realloc(*s, m)
      if tmp.is_null():
        // 尝试最小分配
        m = i + k + 2
        tmp = realloc(*s, m)
        if tmp.is_null():
          // 分配失败: 尽可能复制已有数据
          k = *n - i
          ptr::copy_nonoverlapping((*f).rpos, (*s).add(i), k)
          (*f).rpos = (*f).rpos.add(k)
          (*f).mode |= (*f).mode - 1
          (*f).flags |= F_ERR
          FUNLOCK(f)
          set_errno(ENOMEM)
          return -1
      *s = tmp; *n = m

    // 步骤 3: 批量拷贝可用数据
    if k > 0:
      ptr::copy_nonoverlapping((*f).rpos, (*s).add(i), k)
      (*f).rpos = (*f).rpos.add(k)
      i += k

    // 步骤 4: 找到分隔符则终止
    if !z.is_null(): break

    // 步骤 5: 逐字符读取
    c = getc_unlocked(f)
    if c == EOF:
      if i == 0 || !feof(f):
        FUNLOCK(f)
        return -1
      break                                  // EOF 且已读取数据, 接受当前内容
    // 若下一个字符无法放入当前缓冲，将其推回
    if i + 1 >= *n:
      (*f).rpos = (*f).rpos.sub(1)
      *(*f).rpos = c as u8                   // 推回字符
    else if { *((*s).add(i)) = c as c_char; i += 1; c as u8 == delim as u8 }:
      break                                   // 存入并检查分隔符
  endloop

  // 终止字符串
  *((*s).add(i)) = 0  // '\0'

  FUNLOCK(f)
  return i as isize
```

时间复杂度 O(n)，n 为实际读取的字符数（含 realloc 开销）。

---

## 依赖图

```
getdelim (Public)
  ├── FLOCK / FUNLOCK (锁宏)
  ├── memchr (from <string.h> 或 core::slice::memchr)
  ├── ptr::copy_nonoverlapping (替代 memcpy)
  ├── realloc / alloc (from alloc 模块)
  ├── getc_unlocked (内部宏/内联函数)
  │     └── __uflow (see __uflow spec)
  ├── feof (内部宏)
  ├── set_errno / EINVAL / ENOMEM (from errno 模块)
  └── weak_alias → __getdelim
```

---

## [RELY]

- `FLOCK(f)` / `FUNLOCK(f)` — 条件加锁/解锁。
- `memchr` — 搜索分隔符（可使用 `core::slice::memchr` 或已有实现）。
- `ptr::copy_nonoverlapping` — 批量拷贝（替代 C `memcpy`）。
- `realloc` / `alloc` — 动态内存重分配（rusl 内部 alloc 模块，需兼容 `no_std`）。
- `getc_unlocked(f)` — 无锁字符读取函数。
- `feof(f)` — 检测 EOF 标志。
- `set_errno(val)` / `EINVAL` / `ENOMEM` — 错误报告（errno 模块）。
- `SIZE_MAX` — 防止溢出（`usize::MAX`）。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn getdelim(s: *mut *mut c_char, n: *mut usize, delim: c_int, f: *mut FILE) -> isize;`
  `extern "C" fn __getdelim(s: *mut *mut c_char, n: *mut usize, delim: c_int, f: *mut FILE) -> isize;`

本模块保证对外提供上述两个 ABI 兼容的函数符号，行为符合 POSIX.1-2008 `getdelim` 语义。`__getdelim` 与 `getdelim` 行为完全一致，为弱别名关系。
