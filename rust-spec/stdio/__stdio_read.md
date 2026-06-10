# __stdio_read 函数规约

## 复杂度分级: Level 3

> musl libc 内部 FILE 默认读操作实现。作为 `f->read` 函数指针的默认值，通过 `readv`/`read` 系统调用从文件描述符读取数据，并提供缓冲管理。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};

extern "C" fn __stdio_read(f: *mut FILE, buf: *mut u8, len: usize) -> usize;
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。作为 `f->read` 函数指针的默认值，被 `__toread`、`__uflow` 等间接调用。

> 注意：C 原型中返回 `size_t`，Rust 中使用 `usize`，二者在 ABI 层面等价。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: `*mut FILE`，非空指针，其 `fd` 为有效的文件描述符
- `buf`: `*mut u8`，用户提供的可写缓冲区，长度至少为 `len`
- `len`: 请求读取的字节数

**[Post-condition]:**

**Case 1: 成功读取（cnt > 0）**
- 若无内部缓冲区（`f->buf_size == 0`）或读取的数据全部在 `buf` 内：
  - 返回 `min(cnt, len)`（实际读入 `buf` 的字节数）
- 若有内部缓冲区且 `readv` 将末尾数据填入了内部缓冲区：
  - `f->rpos = f->buf`，`f->rend = f->buf + (cnt - iov[0].iov_len)`
  - 若有缓冲数据（`f->buf_size > 0`），`buf[len-1] = *f->rpos++`
  - 返回 `len`
  - 注意：当 `len-1` 字节读入 `buf` 后，最后一个字节来自内部缓冲区，这是为了翻新 `rpos`/`rend` 指针以建立读取缓冲状态

**Case 2: 读取错误 / EOF（cnt <= 0）**
- `cnt < 0`（读取错误）：设置 `(*f).flags |= F_ERR`
- `cnt == 0`（EOF）：设置 `(*f).flags |= F_EOF`
- 两种情况均返回 `0`

---

### 不变量

**[Invariant]:**
- `f->rpos` 和 `f->rend` 仅在 `cnt > iov[0].iov_len` 时被修改（即 `readv` 填入了内部缓冲区）
- `len - (1 if f->buf_size != 0 else 0)` 确保当流拥有内部缓冲区时，用户缓冲区末尾预留 1 字节给内部缓冲区交错
- 若有内部缓冲区（`f->buf_size > 0`），返回 `len` 时 `buf[len-1]` 被来自内部缓冲区的值覆盖
- 读取错误或 EOF 后，`f->flags` 中对应的标志位被设置，后续读取将直接返回 0

---

### 意图

从文件描述符读取数据到用户缓冲区。当流有内部缓冲区时，使用 `readv` 执行一次分散读取——将尽可能多的数据读入用户缓冲区，将剩余数据读入内部缓冲区（为后续短读取做准备，如 `fgetc`）。若无内部缓冲区，退化为普通 `read`。

Rust 侧实现：
- `iovec` 结构体定义为 `#[repr(C)]` 并与 C 侧兼容
- `readv`/`read` 系统调用通过 `syscall!` 宏实现
- 内部缓冲区指针（`rpos`/`rend`/`buf`）操作使用 `*mut u8` 算术
- 错误标志设置使用位或操作：`f.flags |= F_ERR` 或 `F_EOF`
- 返回值逻辑：C 的 `!!f->buf_size`（非零检测）对应 Rust 的 `(f.buf_size != 0) as usize`
- 内部可将 `readv` 的 `iovec` 构造封装为安全的辅助函数

---

### 系统算法

```
__stdio_read(f, buf, len):
  /* 1. 构造 iovec */
  iov = [IoVec; 2]
  iov[0] = { buf, len - (f.buf_size != 0) as usize }  // 预留1字节给内部缓冲区
  iov[1] = { f.buf, f.buf_size }                       // 内部缓冲区

  /* 2. 系统调用 */
  if iov[0].iov_len > 0:
    cnt = syscall!(SYS_readv, f.fd, iov.as_ptr(), 2)
  else:
    cnt = syscall!(SYS_read, f.fd, iov[1].iov_base, iov[1].iov_len)

  /* 3. 错误处理 */
  if cnt <= 0:
    f.flags |= if cnt != 0 { F_ERR } else { F_EOF }
    return 0

  /* 4. 若无溢出到内部缓冲区，直接返回 */
  if cnt <= iov[0].iov_len as isize:
    return cnt as usize

  /* 5. 设置内部读取缓冲区指针 */
  cnt -= iov[0].iov_len as isize
  f.rpos = f.buf
  f.rend = f.buf.add(cnt as usize)
  if f.buf_size != 0:
    buf[len-1] = *f.rpos   // 回填最后一字节
    f.rpos = f.rpos.add(1)
  return len
```

时间复杂度 O(1)（不含系统调用 I/O 开销）。

---

## 依赖图

```
__stdio_read
  ├─> IoVec (平台相关 repr(C) 结构体)
  ├─> syscall!(SYS_readv)   (内核)
  └─> syscall!(SYS_read)    (内核)
```

---

## [RELY]

- `syscall!` 宏 — 系统调用接口（`SYS_readv`、`SYS_read`）
- `IoVec` — 散布/聚集 I/O 向量（`#[repr(C)]`，与 C 的 `struct iovec` 兼容）
- 常量: `F_ERR`, `F_EOF`

## [GUARANTEE]

Exported Interface:
  `extern "C" fn __stdio_read(f: *mut FILE, buf: *mut u8, len: usize) -> usize;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为与原 C 实现完全一致：通过 `readv`/`read` 系统调用执行缓冲读取，正确设置 `f->rpos`/`f->rend` 指针，正确处理错误/EOF 标志位。
