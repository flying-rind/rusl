# fread 函数规约

## 复杂度分级: Level 1

> musl libc 标准 IO 二进制块读取实现。从 FILE 流中读取指定数量的元素到用户缓冲区，提供 `fread`（加锁）和 `fread_unlocked`（免锁）两个接口。

---

## 函数接口

```rust
use core::ffi::c_void;

// FILE 为 opaque 类型，定义于 rusl-internal 模块

// fread: 加锁版本
extern "C" fn fread(destv: *mut c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;

// fread_unlocked: 弱别名，与 fread 共享同一实现
// weak_alias: fread_unlocked 是 fread 的弱别名
extern "C" fn fread_unlocked(destv: *mut c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;
```

[Visibility]:
- `fread`: User — `<stdio.h>` 标准库函数，用户程序可直接调用。
- `fread_unlocked`: User — 标准库函数，语义上的免锁版本（与 musl 中 `fread` 共享同一实现，musl 的 fread 本身即为线程安全的加锁版本；此弱别名存在是为了 POSIX 兼容）。

两者均必须保持 ABI 兼容：`extern "C"` 导出，参数类型布局与原 C 接口一致。

---

### 前置/后置条件

**[Pre-condition]:**
- `destv`: 非空指针（除非 `size` 或 `nmemb` 为 0），指向至少 `size * nmemb` 字节的有效可写内存。
- `size`: 每个元素的大小（字节），可为 0。
- `nmemb`: 要读取的元素数量，可为 0。
- `f`: 非空 `*mut FILE` 指针，指向已打开的流（读模式或可读模式）。

**[Post-condition]:**
- **Case 1 完全成功读取全部元素**
  - 返回 `nmemb`（要求读取的元素总数）。
  - `destv` 中包含完整数据。

- **Case 2 读取部分元素后遇到 EOF 或出错**
  - 返回 `< nmemb`（实际完整读取的元素数，即 `(已读取字节数) / size`）。
  - `destv` 中前 `返回值 * size` 字节有效。
  - FILE 流相关标志位被设置（`F_EOF` 或 `F_ERR`）。

- **Case 3 `size` 为 0**
  - 返回 0，不执行任何读取操作。

**[Error Behavior]:**
- 遇到 EOF 或读取错误时返回 `< nmemb`。调用者需通过 `feof(f)` / `ferror(f)` 区分。

---

### 不变量

**[Invariant]:**
- `f->lock` 在整个执行期间被当前线程持有（FLOCK/FUNLOCK 配对）。
- `dest + (len - l)` 始终等于已写入 dest 的数据位置。
- `l` 始终等于剩余待读字节数。

---

### 意图

从 FILE 流 `f` 中读取 `nmemb` 个大小为 `size` 字节的元素到 `destv` 缓冲区。采用两阶段策略：先耗尽 FILE 内部读缓冲区的已有数据（`rpos` 到 `rend` 之间），再通过底层 `read` 函数直接读取剩余数据，减少内存拷贝。

Rust 侧实现：
- 使用 `unsafe` 块访问 FILE 内部字段（`rpos`、`rend`、`read` 函数指针等）。
- 阶段 1 使用 `ptr::copy_nonoverlapping` 从 FILE 缓冲区拷贝到用户缓冲区。
- 阶段 2 通过 `__toread(f)` 确保 FILE 处于读模式，再调用 `f->read` 函数指针。
- 模式设置技巧 `f->mode |= f->mode-1`：确保 mode 最低位为 1（读模式标志）。
- 使用 `core::cmp::min` 替代 C 的 `MIN` 宏。
- 若乘法 `size * nmemb` 可能溢出，Rust 中建议使用 `size.checked_mul(nmemb).unwrap_or(0)` 以防 panic。

### 系统算法

```
fread(destv: *mut c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize:
  len = size * nmemb                        // 总字节数
  if size == 0: return 0
  l = len                                   // 剩余待读字节数

  FLOCK(f)                                  // 获取 FILE 锁

  (*f).mode |= (*f).mode - 1                // 设置读模式 (mode 最低位变 1)

  dest = destv as *mut u8

  // 阶段 1: 耗尽 FILE 内部读缓冲区
  if (*f).rpos != (*f).rend:
    k = core::cmp::min((*f).rend as usize - (*f).rpos as usize, l)
    ptr::copy_nonoverlapping((*f).rpos, dest, k)
    (*f).rpos = (*f).rpos.add(k)
    dest = dest.add(k)
    l = l - k

  // 阶段 2: 直接读取剩余数据
  while l > 0:
    k = __toread(f)
    if k != 0: k = 0                        // __toread 失败
    else: k = ((*f).read)(f, dest as *mut c_void, l)
    if k == 0:                              // EOF 或错误
      FUNLOCK(f)
      return (len - l) / size               // 返回已完整读取的元素数
    dest = dest.add(k)
    l = l - k

  FUNLOCK(f)
  return nmemb
```

时间复杂度 O(n)，n 为实际读取的字节数。

---

## 依赖图

```
fread (Public)
  ├── FLOCK / FUNLOCK (锁宏, 来自 stdio_impl 模块)
  │     ├── __lockfile (see __lockfile spec)
  │     └── __unlockfile (see __unlockfile spec)
  ├── ptr::copy_nonoverlapping (替代 memcpy)
  ├── __toread (see __toread spec)
  └── f->read (FILE 函数指针, 通常指向 __stdio_read)
```

---

## [RELY]

- `FLOCK(f)` / `FUNLOCK(f)` — 条件加锁/解锁。
- `ptr::copy_nonoverlapping` — 内存拷贝（替代 C `memcpy`）。
- `__toread(*mut FILE)` — 确保 FILE 处于读模式。
- `f->read` — FILE 底层读函数指针（通常指向 `__stdio_read`）。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn fread(destv: *mut c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;`
  `extern "C" fn fread_unlocked(destv: *mut c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;`

本模块保证对外提供上述两个 ABI 兼容的函数符号，行为符合 ISO C 标准 `fread` 语义。`fread_unlocked` 与 `fread` 行为完全一致，为弱别名关系。
