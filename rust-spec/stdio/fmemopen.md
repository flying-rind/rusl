# fmemopen 函数规约

## 复杂度分级: Level 1

> musl libc 内存流打开函数。创建一个将内存缓冲区作为文件进行读写操作的 `FILE*` 流。

---

## 函数接口

```rust
use core::ffi::{c_int, c_char};

// FILE 为 opaque 类型（定义同 fclose.rs spec）
#[repr(C)]
pub struct FILE { _private: [u8; 0] }

/// 创建一个内存流 FILE 对象。
/// - buf: 用户提供的缓冲区指针（可为 NULL，此时内部分配）
/// - size: 缓冲区大小
/// - mode: 模式字符串，首字符必须为 'r'/'w'/'a'，可选含 '+' 表示可读写
/// 返回新创建的 FILE 指针，失败返回 NULL。
unsafe extern "C" fn fmemopen(
    buf: *mut c_void,
    size: usize,
    mode: *const c_char,
) -> *mut FILE;
```

[Visibility]: `fmemopen` 声明于 `<stdio.h>`，是 POSIX.1-2008 标准接口，用户可直接调用。在编译产物中以 `#[no_mangle]` 导出 `fmemopen` 符号，必须保持 ABI 兼容。

---

### 内部类型设计（无需 ABI 兼容，可用安全 Rust 重新设计）

以下内部类型在原 C 实现中为 `struct cookie`、`struct mem_FILE` 和静态函数 `mread`/`mwrite`/`mseek`/`mclose`。在 Rust 侧可完全重新设计，利用 Rust 类型系统和安全抽象。

#### 内部状态结构

```rust
/// 内存流的内部状态控制块
/// 对应原 C 的 struct cookie
#[derive(Debug)]
struct MemCookie {
    /// 当前读写位置
    pos: usize,
    /// 有效数据长度
    len: usize,
    /// 缓冲区总大小
    size: usize,
    /// 缓冲区指针（指向用户提供或内部分配的缓冲区）
    buf: *mut u8,
    /// 打开模式首字符（'r' / 'w' / 'a'）
    mode: u8,
}
```

注意：`buf` 字段保持裸指针是必要的，因为用户提供的缓冲区不由本模块管理生命周期。但在 Rust 侧内部操作中可封装为安全切片引用（`&[u8]` / `&mut [u8]`，通过 `core::slice::from_raw_parts` / `from_raw_parts_mut`）。

#### 内部 FILE 包装

```rust
/// 内存流 FILE 对象
/// 对应原 C 的 struct mem_FILE
/// 内部使用 Box 管理堆分配，实现 RAII
struct MemFile {
    /// 标准 FILE 结构（ABI 兼容部分）
    file: FILE,
    /// cookie 状态
    cookie: MemCookie,
    /// 预留 UNGET 字节 + BUFSIZ 字节的 FILE 内部缓冲区
    buf: [u8; UNGET + BUFSIZ],
    /// 灵活数组缓冲区 buf2（仅在用户未提供 buf 时用作自动分配的数据存储）
    /// 在 Rust 中用 Vec<u8> 替代 C 的 flexible array member
    internal_buf: Vec<u8>,
}
```

[Visibility]: `MemCookie`、`MemFile` 及所有内部函数均为模块私有（`pub(crate)` 可见性），不对外暴露。

---

### 前置/后置条件

**对外接口 `fmemopen`:**

**[Pre-condition]:**
- `mode`: 有效的模式字符串，首字符必须为 `'r'`、`'w'` 或 `'a'`；可选含 `'+'` 表示可读写
- `buf`: 用户提供的缓冲区指针（可为 `NULL`，此时内部分配）
- `size`: 缓冲区大小（若 `buf` 非 `NULL` 则为用户提供的缓冲区大小；若 `buf` 为 `NULL` 则分配 `size` 字节）
- 若 `buf` 为 `NULL` 且 `size > isize::MAX`，设置 `errno = ENOMEM` 并返回 `NULL`

**[Post-condition]:**

- **Case 1: 成功** — 返回指向新创建的 `FILE` 对象的指针
  - `FILE` 的 `fd` 设置为 `-1`（表示无底层文件描述符）
  - `FILE` 的 `read`、`write`、`seek`、`close` 函数指针设置为内部实现
  - 若 mode 不含 `'+'`，设置只读/只写限制标志（`F_NOWR` 或 `F_NORD`）
  - 若 `buf` 为 `NULL`，内部分配 `size` 字节缓冲区并零填充
  - 若 mode 为 `'r'`：`len = size`（文件内容为整个缓冲区初始内容）
  - 若 mode 为 `'a'`：`len = pos = strnlen(buf, size)`（从末尾开始追加），且 `'+'` 模式下设置 `*buf = 0`（确保字符串终止）
  - 若 `'+'` 模式且 mode 首字符为 `'w'`：设置 `*buf = 0`（创建空字符串）
  - 通过 `__ofl_add` 注册到全局打开文件链表
  - 若未启用线程化，禁用 FILE 对象锁

- **Case 2: 失败** — 返回 `NULL`
  - 若 mode 首字符不合法：`errno = EINVAL`
  - 若 `buf` 为 `NULL` 且 `size > isize::MAX`：`errno = ENOMEM`
  - 若内存分配失败：保持 `alloc` 模块设置的错误

**[Error Behavior]:**

| 条件 | errno 值 |
|------|----------|
| mode 首字符非 `r`/`w`/`a` | `EINVAL` |
| `buf == NULL` 且 `size > isize::MAX` | `ENOMEM` |
| 内存分配失败 | 由 alloc 模块设置 |

---

### 不变量

**[Invariant]:**
- `fd` 始终为 `-1`（无底层文件描述符，所有 I/O 操作直接操作内存）
- 内存流不支持行缓冲（`lbf = EOF`）
- 关闭操作（内部 `close` 回调）不释放用户提供的 `buf`（调用者负责管理）
- 对于 `fmemopen` 内部分配的缓冲区，其生命周期与 `MemFile` 绑定，由 `fclose` 整体释放
- 当 `'+'` 模式激活时，`fmemopen` 始终尝试维护一个以 NULL 结尾的字符串

---

### 意图

创建一个使用内存缓冲区进行 I/O 的 `FILE` 流。允许程序将内存当做文件读写，适用于需要在内存中进行数据格式化的场景。

Rust 侧实现：
- 外部 `fmemopen` 接口保持 `unsafe extern "C"` 的 ABI 签名
- 内部 `MemFile` 使用 `Box` 管理堆分配生命周期，内部缓冲区使用 `Vec<u8>` 替代 C 的 flexible array member
- `read`/`write`/`seek`/`close` 内部回调函数在 Rust 侧设计为安全函数，通过 `&mut MemCookie` 操作状态，不直接操作裸指针
- 模式字符串解析使用 Rust 的 `match` 或字节匹配，替代 `strchr`
- 内存复制使用 `core::ptr::copy` / `core::slice` 的方法，避免手动 `memcpy`

### 系统算法

```
fmemopen(buf, size, mode):
  1. plus = mode 中是否含有 '+'
  2. 校验 mode 首字符: 必须为 'r'/'w'/'a', 否则 errno=EINVAL, return NULL
  3. 若 buf.is_null() 且 size > isize::MAX: errno=ENOMEM, return NULL
  4. 分配 MemFile: Box::new(MemFile { ... })
     若分配失败: return NULL
  5. 初始化 FILE 字段:
        f.cookie = &mut f.cookie as *mut _ as *mut c_void
        f.fd = -1
        f.lbf = EOF
        f.buf = &f.buf[UNGET..] 的指针
        f.buf_size = sizeof(f.buf) - UNGET
  6. 若 buf.is_null(): f.internal_buf = vec![0u8; size]; buf = f.internal_buf.as_mut_ptr()
  7. 初始化 cookie:
        c.buf = buf; c.size = size; c.mode = *mode
  8. 设置文件标志和初始状态:
        若 !plus: f.flags = if *mode == 'r' { F_NOWR } else { F_NORD }
        若 *mode == 'r': c.len = size
        若 *mode == 'a': c.len = c.pos = strnlen(buf, size)
            若 plus: *c.buf = 0
        若 plus 且 *mode == 'w': *c.buf = 0
  9. 设置操作函数指针:
        f.read  = mem_read   (内部安全函数)
        f.write = mem_write  (内部安全函数)
        f.seek  = mem_seek   (内部安全函数)
        f.close = mem_close  (内部安全函数)
  10. 若 !threaded: f.lock = -1
  11. return __ofl_add(&f.file)
```

---

### 内部回调函数设计（Rust 安全重构）

以下内部函数在 C 侧为 `static` 函数。在 Rust 侧可重新设计为模块私有（`pub(crate)`）的安全/少 unsafe 函数。

#### mem_read (对应 C 的 mread)

```rust
/// 从内存流缓冲区读取数据
/// 输入: cookie 的可变引用、输出缓冲区切片、请求长度
/// 返回: 实际读取字节数
fn mem_read(cookie: &mut MemCookie, buf: &mut [u8], len: usize) -> usize
```

**算法:**
```
mem_read(cookie, buf, len):
  rem = cookie.len.saturating_sub(cookie.pos)    // 剩余可读字节数
  if len > rem:
    len = rem
    设置 F_EOF 标志
  buf[..len].copy_from_slice(&cookie.buf[cookie.pos..][..len])
  cookie.pos += len
  rem -= len
  // 预读: 将剩余数据预读到 FILE 内部缓冲区
  if rem > 0:
    preload = min(rem, f.buf_size)
    f.rpos.copy_from_slice(&cookie.buf[cookie.pos..][..preload])
    cookie.pos += preload
  return len
```

#### mem_write (对应 C 的 mwrite)

```rust
/// 将数据写入内存流缓冲区
fn mem_write(cookie: &mut MemCookie, buf: &[u8], len: usize) -> usize
```

**算法:**
```
mem_write(cookie, buf, len):
  // 先刷新 FILE 写缓冲区中的待写数据（由调用者处理 f->wpos/f->wbase）
  if cookie.mode == 'a': cookie.pos = cookie.len  // 追加模式
  rem = cookie.size - cookie.pos
  if len > rem: len = rem
  cookie.buf[cookie.pos..][..len].copy_from_slice(buf[..len])
  cookie.pos += len
  if cookie.pos > cookie.len:
    cookie.len = cookie.pos
    if cookie.len < cookie.size: cookie.buf[cookie.len] = 0  // 维护 NULL 终止
    else if F_NORD && cookie.size > 0: cookie.buf[cookie.size-1] = 0
  return len
```

#### mem_seek (对应 C 的 mseek)

```rust
/// 在内存流缓冲区中移动读写位置
fn mem_seek(cookie: &mut MemCookie, off: i64, whence: c_int) -> Result<usize, ()>
```

**算法:**
```
mem_seek(cookie, off, whence):
  if whence > 2: return Err(())  // errno = EINVAL
  base = match whence {
    0 => 0,          // SEEK_SET
    1 => cookie.pos, // SEEK_CUR
    2 => cookie.len, // SEEK_END
    _ => unreachable!(),
  }
  if off < -(base as i64) || off > (cookie.size as i64) - (base as i64):
    return Err(())    // errno = EINVAL
  cookie.pos = (base as i64 + off) as usize
  return Ok(cookie.pos)
```

#### mem_close (对应 C 的 mclose)

```rust
/// 内存流关闭操作
/// 始终成功返回 0，不释放 cookie.buf
fn mem_close(_cookie: &MemCookie) -> c_int { 0 }
```

---

## 依赖图

```
fmemopen
  ├─> MemCookie (struct)       (内部定义)
  ├─> MemFile (struct)         (内部定义)
  ├─> mem_read                 (内部定义, pub(crate))
  ├─> mem_write                (内部定义, pub(crate))
  ├─> mem_seek                 (内部定义, pub(crate))
  ├─> mem_close                (内部定义, pub(crate))
  ├─> __ofl_add                (see ofl_add.rs spec — 注册到全局打开文件链表)
  ├─> alloc 模块               (Box/Vec 分配, 替代 C 的 malloc/memset/memcpy)
  └─> strchr / strnlen         (from rusl-string 或内联实现)
```

---

## [RELY]

- `__ofl_add`: 将新 FILE 注册到全局打开文件链表（定义于 `rusl-stdio` 的 `ofl_add` 模块）
- `rusl-string` 的 `strchr` / `strnlen`（或内联等价实现）
- Rust `alloc` 模块：`Box`、`Vec` 用于内存管理，替代 C 的 `malloc`/`memset`/`memcpy`
- 线程状态 (`libc.threaded`)：由 `rusl-internal` 提供

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn fmemopen(
    buf: *mut c_void,
    size: usize,
    mode: *const c_char,
) -> *mut FILE;
```

本模块保证对外提供 ABI 兼容的 `fmemopen` 符号。行为符合 POSIX.1-2008 `fmemopen()` 语义：创建内存流 `FILE` 对象，设置正确的读写模式和初始状态，返回可供标准 I/O 函数使用的 `FILE*` 指针。

内部所有辅助函数（`mem_read`、`mem_write`、`mem_seek`、`mem_close`）及结构体（`MemCookie`、`MemFile`）均为模块私有，不对外暴露。
