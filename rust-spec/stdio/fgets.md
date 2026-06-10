# fgets 函数规约

## 复杂度分级: Level 1

> musl libc 标准 IO 行读取实现。从 FILE 流中读取一行字符串（包括换行符）到用户缓冲区。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};

// FILE 为 opaque 类型，定义于 rusl-internal 模块

// fgets: 加锁版本
extern "C" fn fgets(s: *mut c_char, n: c_int, f: *mut FILE) -> *mut c_char;

// fgets_unlocked: 弱别名，与 fgets 共享同一实现
// weak_alias: fgets_unlocked 是 fgets 的弱别名
extern "C" fn fgets_unlocked(s: *mut c_char, n: c_int, f: *mut FILE) -> *mut c_char;
```

[Visibility]:
- `fgets`: User — `<stdio.h>` 标准库函数，用户程序可直接调用。
- `fgets_unlocked`: User — POSIX 免锁 `fgets`，在 musl 中与 `fgets` 共享同一实现。

两者均必须保持 ABI 兼容：`extern "C"` 导出，参数类型布局与原 C 接口一致。

---

### 前置/后置条件

**[Pre-condition]:**
- `s`: 非空指针，指向至少 `n` 字节的有效可写内存。
- `n`: 缓冲区大小（`c_int`），至少为 1（否则无操作返回 `null_mut()`）。
- `f`: 非空 `*mut FILE` 指针，指向已打开的读模式流。

**[Post-condition]:**
- **Case 1 成功读取一行（包括换行符）**
  - 返回 `s`（指向用户缓冲区）。
  - `s` 中以 `\0` 结尾，包含 `\n` 换行符（若在 `n-1` 字符内遇到）。
  - 实际读取字符数 `<= n-1`。

- **Case 2 成功读取但未遇到换行（缓冲区满）**
  - 返回 `s`。
  - `s` 中包含 `n-1` 个字符 + `\0`。
  - 下一次 `fgets` 调用将继续读取后续字符。

- **Case 3 到达文件末尾且未读取任何字符**
  - 返回 `null_mut()`。
  - `s` 内容不变。
  - FILE 流设置 `F_EOF` 标志。

- **Case 4 `n < 1`**
  - 返回 `null_mut()`，不执行任何操作。

- **Case 5 读取错误（且未读取到任何字符）**
  - 返回 `null_mut()`。

- **Case 6 `n == 1`**
  - 返回 `s`，仅写入 `\0`（空字符串）。

**[Error Behavior]:**
- 到达 EOF 且未读取任何字符时返回 `null_mut()`。调用者需检查返回值。

---

### 不变量

**[Invariant]:**
- 循环中 `p - s` 始终等于已存入缓冲区的字符数。
- `n` 始终等于剩余可写入字符数（不含结尾 `\0`）。
- FILE 锁在 `fgets` 执行期间被持有（`fgets_unlocked` 不持有锁，但 musl 中二者实现相同，均通过 FLOCK/FUNLOCK 加锁）。

---

### 意图

从 FILE 流 `f` 中读取至多 `n-1` 个字符到缓冲区 `s`，遇到换行符 `\n` 或文件末尾时停止。读取的字符串总是以 `\0` 结尾（`n>=1` 时）。换行符会被保留在缓冲区中（若遇到）。

Rust 侧实现：
- 采用两阶段策略：
  1. **批量拷贝**: 先利用 FILE 内部读缓冲区，用 `memchr` 快速定位 `\n`，用 `ptr::copy_nonoverlapping` 批量拷贝（替代 C 的 `memcpy`）。
  2. **逐字符读取**: 缓冲区耗尽后逐字符读取，效率较低但确保正确性。
- 使用 `unsafe` 块访问 FILE 内部字段和操作用户缓冲区。
- `fgets_unlocked` 在 Rust 侧通过复制函数体实现，或使用 linker 脚本实现真正的弱符号别名。

### 系统算法

```
fgets(s: *mut c_char, n: c_int, f: *mut FILE) -> *mut c_char:
  p = s
  FLOCK(f)

  // 边界处理: n <= 1
  if n <= 1:
    (*f).mode |= (*f).mode - 1
    FUNLOCK(f)
    if n < 1: return null_mut()
    *p = 0  // '\0'
    return s

  n = n - 1                                // 为 '\0' 保留一个位置

  // 主循环
  while n > 0:
    // 阶段 1: 尽可能从 FILE 缓冲区批量拷贝
    if (*f).rpos != (*f).rend:
      z = memchr((*f).rpos, '\n' as c_int, (*f).rend - (*f).rpos)
      k = if z.is_some() { z_offset + 1 } else { (*f).rend - (*f).rpos }
      k = MIN(k, n)
      ptr::copy_nonoverlapping((*f).rpos, p as *mut u8, k)
      (*f).rpos = (*f).rpos.add(k)
      p = p.add(k)
      n = n - k
      if z.is_some() || n == 0: break

    // 阶段 2: 逐字符读取(缓冲区已空)
    c = getc_unlocked(f)
    if c < 0:
      if p == s || !feof(f): s = null_mut()
      break
    n = n - 1
    *p = c as c_char
    p = p.add(1)
    if c as u8 == '\n' as u8: break

  // 终止字符串
  if !s.is_null(): *p = 0  // '\0'

  FUNLOCK(f)
  return s
```

时间复杂度 O(n)，n 为实际读取的字符数。

---

## 依赖图

```
fgets (Public)
  ├── FLOCK / FUNLOCK (锁宏, 来自 stdio_impl 模块)
  ├── memchr (from <string.h> 或 core::slice::memchr)
  ├── ptr::copy_nonoverlapping (替代 memcpy)
  ├── getc_unlocked (内部宏/内联函数)
  │     └── __uflow (see __uflow spec)
  ├── feof (内部宏)
  └── weak_alias → fgets_unlocked
```

---

## [RELY]

- `FLOCK(f)` / `FUNLOCK(f)` — 条件加锁/解锁。
- `memchr` — 在缓冲区中搜索换行符（可使用 `core::slice::memchr` 或已有实现）。
- `ptr::copy_nonoverlapping` — 批量拷贝（替代 C `memcpy`）。
- `getc_unlocked(f)` — 无锁字符读取函数。
- `feof(f)` — 检测 EOF 标志。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn fgets(s: *mut c_char, n: c_int, f: *mut FILE) -> *mut c_char;`
  `extern "C" fn fgets_unlocked(s: *mut c_char, n: c_int, f: *mut FILE) -> *mut c_char;`

本模块保证对外提供上述两个 ABI 兼容的函数符号，行为符合 ISO C 标准 `fgets` 语义。`fgets_unlocked` 与 `fgets` 行为完全一致，为弱别名关系。
