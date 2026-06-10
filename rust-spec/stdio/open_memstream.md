# open_memstream 函数规约

## 复杂度分级: Level 1

> musl libc 动态内存流打开函数。创建一个自动增长的只写流，允许程序动态构建内存中的字符串缓冲区，并通过输出参数获取最终缓冲区指针和大小。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为 opaque 类型（定义同 fclose.rs spec）
#[repr(C)]
pub struct FILE { _private: [u8; 0] }

/// 创建动态内存流。
/// - bufp: 输出参数，指向调用者的 char* 变量，关闭时写入最终缓冲区地址
/// - sizep: 输出参数，指向调用者的 size_t 变量，实时更新缓冲区大小（不含 NULL 终止符）
/// 返回新创建的只写 FILE 指针，失败返回 NULL。
unsafe extern "C" fn open_memstream(
    bufp: *mut *mut u8,   // char **bufp
    sizep: *mut usize,     // size_t *sizep
) -> *mut FILE;
```

[Visibility]: `open_memstream` 声明于 `<stdio.h>`，是 POSIX.1-2008 标准接口，属于 GNU 扩展。在编译产物中以 `#[no_mangle]` 导出 `open_memstream` 符号，必须保持 ABI 兼容。

---

### 内部类型设计（无需 ABI 兼容，可用安全 Rust 重新设计）

#### 内部状态结构

```rust
/// 动态内存流的内部状态控制块
/// 对应原 C 的 struct cookie
struct MsCookie {
    /// 指向调用者缓冲区指针的指针（输出参数，关闭时最终更新）
    bufp: *mut *mut u8,
    /// 指向调用者大小变量的指针（输出参数，实时更新）
    sizep: *mut usize,
    /// 当前写入位置
    pos: usize,
    /// 内部分配的缓冲区（使用 Vec 替代裸指针 + 手动 realloc）
    buf: Vec<u8>,
    /// 当前有效数据长度
    len: usize,
}
```

注意：由于 `bufp` 和 `sizep` 指向调用者的变量，它们是裸指针以避免生命周期束缚。但内部缓冲区 `buf` 使用 `Vec<u8>` 以利用 Rust 的安全动态内存管理（自动 `realloc`，避免手动调用 C 的 `realloc`/`free`）。

#### 内部 FILE 包装

```rust
/// 动态内存流 FILE 对象
/// 对应原 C 的 struct ms_FILE
struct MsFile {
    /// 标准 FILE 结构（ABI 兼容部分）
    file: FILE,
    /// cookie 状态
    cookie: MsCookie,
    /// BUFSIZ 字节的 FILE 内部缓冲区
    buf: [u8; BUFSIZ],
}
```

[Visibility]: `MsCookie`、`MsFile` 及所有内部函数均为模块私有（`pub(crate)` 可见性），不对外暴露。

---

### 前置/后置条件

**对外接口 `open_memstream`:**

**[Pre-condition]:**
- `bufp`: 指向 `*mut u8` 的指针（非 `NULL`），将在此处返回最终缓冲区指针
- `sizep`: 指向 `usize` 的指针（非 `NULL`），将在此处返回缓冲区大小（不包含 NULL 终止符）

**[Post-condition]:**

- **Case 1: 成功** — 返回新创建的 `FILE*` 对象
  - 流已设置为只写模式（`F_NORD` 标志 — 不允许读取）
  - `fd` 设为 `-1`（无底层文件描述符）
  - `mode` 设为 `-1`（特殊值，表示内存流）
  - 流禁止行缓冲（`lbf = EOF`）
  - 初始缓冲区大小为 1 字节（`buf = [0]`，分配了空字符串）
  - `*sizep = 0`，`*bufp` 指向初始缓冲区
  - 流的 `write` 回调为内部 `memstream_write`（自动动态增长）
  - 流的 `seek` 回调为内部 `memstream_seek`（内存中 seek）
  - 流的 `close` 回调为内部 `memstream_close`（不释放缓冲区，留给调用者）
  - 通过 `__ofl_add` 注册到全局打开文件链表
  - 若未启用线程化，禁用 FILE 对象锁

- **Case 2: 失败** — 返回 `NULL`
  - 若初始内存分配失败

**[Error Behavior]:**
- `open_memstream` 本身仅在初始分配失败时返回 `NULL`
- 后续写入的 `realloc` 失败不会导致 `open_memstream` 本身失败（流已成功打开），而是产生短写入（返回 `0`）

---

### 不变量

**[Invariant]:**
- `fd` 始终为 `-1`（无底层文件描述符）
- 流是只写的（通过 `F_NORD` 标志禁止读取）
- `*sizep` 始终反映当前缓冲区有效数据长度（不断更新，实时可见）
- 缓冲区始终以 NULL 字符终止（`buf[len] = 0` 或 `buf[0] = 0`）
- 关闭后 `*bufp` 指向的缓冲区归调用者所有，调用者负责 `free`
- 初始分配 1 字节，然后自动按需增长

---

### 意图

创建一个只写的动态内存流。写入的数据被动态分配到内存缓冲区中，该缓冲区根据需要自动增长。调用者通过 `bufp` 和 `sizep` 获取最终的缓冲区指针和大小。当流被 `fclose` 关闭时，缓冲区被终止为有效的 C 字符串，并且 `*bufp` 和 `*sizep` 被更新为最终状态。

Rust 侧实现：
- 外部接口 `open_memstream` 保持 `unsafe extern "C"` 的 ABI 签名
- 内部缓冲区使用 `Vec<u8>` 替代 C 的手动 `malloc`/`realloc`/`free`，利用 Rust 的自动容量管理和安全切片操作
- 缓冲区增长策略：当空间不足时，`Vec` 自动以 2 倍策略增长，或直接 `resize` 到 `max(pos + len + 1, cur_cap)`，确保 NULL 终止符空间
- `*sizep` 和 `*bufp` 的更新集中在 `unsafe` 指针写操作中，其余逻辑使用安全 Rust
- 关闭时，通过 `Vec::leak` 或 `Box::into_raw` 将缓冲区所有权转移给调用者

### 系统算法

```
open_memstream(bufp, sizep):
  1. 分配 MsFile: let f = Box::new(MsFile { ... })
     若分配失败: return NULL
  2. 初始化 cookie:
        c.bufp = bufp       // 指向调用者的 buffer 指针
        c.sizep = sizep     // 指向调用者的 size 指针
        c.pos = 0
        c.len = 0
        c.buf = vec![0u8]   // 初始分配 1 字节，内容为 0（空字符串）
        *sizep = 0
        *bufp = c.buf.as_mut_ptr()
  3. 初始化 FILE:
        f.file.flags = F_NORD
        f.file.fd = -1
        f.file.buf = &f.buf 的指针
        f.file.buf_size = BUFSIZ
        f.file.lbf = EOF
        f.file.write = memstream_write  (内部函数指针)
        f.file.seek = memstream_seek    (内部函数指针)
        f.file.close = memstream_close  (内部函数指针)
        f.file.mode = -1
  4. 若 !threaded: f.file.lock = -1
  5. return __ofl_add(&f.file)
```

时间复杂度 O(1)。

---

### 内部回调函数设计（Rust 安全重构）

以下内部函数在原 C 实现中为 `static` 函数。在 Rust 侧可重新设计为模块私有函数。

#### memstream_write (对应 C 的 ms_write)

```rust
/// 将数据写入动态内存流缓冲区
/// - 先写出 FILE 自身缓冲区中的待写数据（递归）
/// - 若空间不足，自动增长 Vec
/// - 实时更新 *sizep 和 *bufp
fn memstream_write(cookie: &mut MsCookie, buf: &[u8], len: usize) -> usize;
```

**算法:**
```
memstream_write(cookie, buf, len):
  // 先刷新 FILE 写缓冲区中的待写数据（由调用者处理 f.wpos/f.wbase）

  if len + cookie.pos >= cookie.buf.len():       // 需要扩容
    // 新大小: max(2*cur+1, pos+len+1) — 确保留足 NULL 终止符空间
    new_len = (2 * cookie.buf.len() + 1).max(cookie.pos + len + 1)
    cookie.buf.resize(new_len, 0)                // Vec 自动 realloc + 零填充

  cookie.buf[cookie.pos..][..len].copy_from_slice(buf[..len])
  cookie.pos += len
  if cookie.pos > cookie.len:
    cookie.len = cookie.pos
  // 实时更新调用者可见的状态
  unsafe {
    *cookie.sizep = cookie.pos                   // 调用者始终能读到最新大小
    *cookie.bufp = cookie.buf.as_mut_ptr()       // Vec 可能已 realloc，更新指针
  }
  return len
```

**注意**: Rust 的 `Vec` 自动管理容量和增长，但增长策略需明确指定以满足 C 侧的期望行为。使用 `Vec::resize` 可同时完成扩容和零填充，替代 C 的 `realloc` + `memset`。

#### memstream_seek (对应 C 的 ms_seek)

```rust
/// 在动态内存流中移动写入位置
/// 支持 SEEK_SET/SEEK_CUR/SEEK_END
/// 与 fmemopen 的 seek 不同：size 理论上无限（动态增长），但当前位置不能超过 SSIZE_MAX
fn memstream_seek(cookie: &mut MsCookie, off: i64, whence: c_int) -> Result<usize, ()>;
```

**算法:**
```
memstream_seek(cookie, off, whence):
  if whence > 2: errno = EINVAL, return Err(())
  base = match whence {
    0 => 0,          // SEEK_SET
    1 => cookie.pos, // SEEK_CUR
    2 => cookie.len, // SEEK_END
    _ => unreachable!(),
  }
  if off < -(base as i64) || off > SSIZE_MAX - (base as i64):
    errno = EINVAL, return Err(())
  cookie.pos = (base as i64 + off) as usize
  return Ok(cookie.pos)
```

#### memstream_close (对应 C 的 ms_close)

```rust
/// 动态内存流关闭操作
/// 不释放缓冲区（所有权已转移给调用者）
fn memstream_close(cookie: &mut MsCookie) -> c_int;
```

**算法:**
```
memstream_close(cookie):
  // 缓冲区所有权转移给调用者
  // 使用 Vec::leak 或手动管理，确保 buf 不被 drop
  let leaked = cookie.buf.leak();    // 或: mem::forget(cookie.buf)
  // 更新调用者输出参数
  unsafe {
    *cookie.bufp = leaked.as_mut_ptr();
    *cookie.sizep = cookie.len;
  }
  return 0
```

**注意**: Rust 的 `Vec` 在 `drop` 时会释放内存。为了让调用者获取所有权，需使用 `Vec::leak`（或 `ManuallyDrop`）防止自动释放。这使得调用者最终需要调用 `free`（或 `libc::free`）来释放该缓冲区。

---

## 依赖图

```
open_memstream
  ├─> MsCookie (struct)        (内部定义)
  ├─> MsFile (struct)          (内部定义)
  ├─> memstream_write          (内部定义, pub(crate))
  ├─> memstream_seek           (内部定义, pub(crate))
  ├─> memstream_close          (内部定义, pub(crate))
  ├─> __ofl_add                (see ofl_add.rs spec — 注册到全局打开文件链表)
  ├─> alloc 模块               (Vec 分配/增长, 替代 C 的 malloc/realloc/free)
  └─> 线程状态                 (来自 "libc" 内部模块)
```

---

## [RELY]

- `__ofl_add`: 将新 FILE 注册到全局打开文件链表（定义于 `rusl-stdio` 的 `ofl_add` 模块）
- Rust `alloc` 模块：`Vec<u8>` 用于动态内存管理，替代 C 的 `malloc`/`realloc`/`free`/`memset`/`memcpy`
- 线程状态 (`libc.threaded`)：由 `rusl-internal` 提供

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn open_memstream(
    bufp: *mut *mut u8,
    sizep: *mut usize,
) -> *mut FILE;
```

本模块保证对外提供 ABI 兼容的 `open_memstream` 符号。行为符合 POSIX.1-2008 `open_memstream()` 语义：创建只写的动态内存流，写入数据通过 `Vec` 自动增长，`*sizep` 实时反映当前大小，关闭时通过 `*bufp`/`*sizep` 将最终缓冲区指针和大小返回给调用者（调用者负责释放缓冲区）。

内部所有辅助函数（`memstream_write`、`memstream_seek`、`memstream_close`）及结构体（`MsCookie`、`MsFile`）均为模块私有，不对外暴露。
