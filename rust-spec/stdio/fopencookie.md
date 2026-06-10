# fopencookie 函数规约

## 复杂度分级: Level 1

> musl libc 自定义回调流打开函数。创建一个 `FILE*` 流，其底层 I/O 操作由用户提供的回调函数实现。这是 GNU 扩展接口。

---

## 函数接口

```rust
use core::ffi::{c_int, c_char, c_void};

// FILE 为 opaque 类型（定义同 fclose.rs spec）
#[repr(C)]
pub struct FILE { _private: [u8; 0] }

/// cookie_io_functions_t: 用户提供的 I/O 回调函数集合
/// 对应 C 的 cookie_io_functions_t，定义于 <stdio.h>
#[repr(C)]
pub struct cookie_io_functions_t {
    /// 读取回调: (cookie, buf, len) -> 实际读取字节数或负数表示错误
    pub read: Option<unsafe extern "C" fn(*mut c_void, *mut c_char, usize) -> isize>,
    /// 写入回调: (cookie, buf, len) -> 实际写入字节数或负数表示错误
    pub write: Option<unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> isize>,
    /// seek 回调: (cookie, *offset, whence) -> 0 成功, -1 失败
    pub seek: Option<unsafe extern "C" fn(*mut c_void, *mut i64, c_int) -> c_int>,
    /// 关闭回调: (cookie) -> 0 成功, -1 失败
    pub close: Option<unsafe extern "C" fn(*mut c_void) -> c_int>,
}

/// 创建用户自定义回调流。
/// - cookie: 不透明用户数据，传递给所有回调
/// - mode: 模式字符串，首字符必须为 'r'/'w'/'a'，可选含 '+'
/// - iofuncs: 用户提供的 I/O 回调函数集合
/// 返回新创建的 FILE 指针，失败返回 NULL。
unsafe extern "C" fn fopencookie(
    cookie: *mut c_void,
    mode: *const c_char,
    iofuncs: cookie_io_functions_t,
) -> *mut FILE;
```

[Visibility]: `fopencookie` 声明于 `<stdio.h>`（需定义 `_GNU_SOURCE`），是 GNU 扩展接口，用户可直接调用。在编译产物中以 `#[no_mangle]` 导出 `fopencookie` 符号，必须保持 ABI 兼容。

---

### 内部类型设计（无需 ABI 兼容，可用安全 Rust 重新设计）

#### 内部状态结构

```rust
/// 自定义回调流的内部状态
/// 对应原 C 的 struct fcookie
struct FCookie {
    /// 用户提供的不透明 cookie，传递给所有回调函数
    cookie: *mut c_void,
    /// 用户提供的 I/O 回调函数集合
    iofuncs: cookie_io_functions_t,
}
```

#### 内部 FILE 包装

```rust
/// 自定义回调流 FILE 对象
/// 对应原 C 的 struct cookie_FILE
struct CookieFile {
    /// 标准 FILE 结构（ABI 兼容部分）
    file: FILE,
    /// fcookie 状态
    fc: FCookie,
    /// 预留 UNGET 字节 + BUFSIZ 字节的 FILE 内部缓冲区
    buf: [u8; UNGET + BUFSIZ],
}
```

[Visibility]: `FCookie`、`CookieFile` 及所有内部回调封装函数均为模块私有（`pub(crate)` 可见性），不对外暴露。

---

### 前置/后置条件

**对外接口 `fopencookie`:**

**[Pre-condition]:**
- `mode`: 有效的模式字符串，首字符必须为 `'r'`、`'w'` 或 `'a'`；可选含 `'+'` 表示可读写
- `iofuncs`: 用户提供的回调函数集合，各个函数可以为 `NULL`（表示该操作不被支持）
- `cookie`: 不透明用户数据指针（可为任意值，被传递给所有回调）

**[Post-condition]:**

- **Case 1: 成功** — 返回新创建的 `FILE*` 对象
  - `FILE` 的 `fd` 设为 `-1`（无底层文件描述符）
  - `FILE` 的 `cookie` 字段指向内部 `FCookie` 结构
  - `FILE` 的 `read`、`write`、`seek`、`close` 函数指针设置为内部封装函数
  - 内部封装函数转发调用到用户提供的回调，或将不支持的操作用适当的默认行为处理
  - 若 mode 不含 `'+'`，设置只读/只写限制标志（`F_NOWR` 或 `F_NORD`）
  - `buf_size = BUFSIZ`（`sizeof(f.buf) - UNGET`），支持用户缓冲
  - 行缓冲被禁用（`lbf = EOF`）
  - 通过 `__ofl_add` 注册到全局打开文件链表

- **Case 2: 失败** — 返回 `NULL`
  - 若 mode 首字符不合法：`errno = EINVAL`
  - 若内存分配失败：`errno = ENOMEM`

**[Error Behavior]:**

| 条件 | errno 值 |
|------|----------|
| mode 首字符非 `r`/`w`/`a` | `EINVAL` |
| 内存分配失败 | `ENOMEM` |

---

### 不变量

**[Invariant]:**
- `fd` 始终为 `-1`（无底层文件描述符）
- 不支持行缓冲（`lbf = EOF`）
- 关闭操作（内部 `close` 回调封装）调用用户提供的 `close` 回调（若为 `NULL` 则返回 `0`）

---

### 意图

创建一个 `FILE*` 流，其所有底层 I/O 操作都由用户提供的回调函数执行。`cookie` 参数是不透明的用户数据，被传递给每个回调函数。这允许程序将任意数据源或目标伪装为文件流，实现自定义 I/O 后端。

Rust 侧实现：
- 外部接口 `fopencookie` 保持 `unsafe extern "C"` 的 ABI 签名
- `cookie_io_functions_t` 保持 `#[repr(C)]` 布局，以确保与 C 侧传递的结构体内存兼容
- 内部 `CookieFile` 使用 `Box` 管理堆分配生命周期
- 内部封装函数（对应原 C 的 `cookieread`/`cookiewrite`/`cookieseek`/`cookieclose`）在 Rust 侧设计为模块私有函数，将用户裸函数指针调用封在 `unsafe` 块内，其余逻辑用安全 Rust 实现
- 通过 `Option` 类型表示可空的回调（替代 C 的 NULL 检查）

### 系统算法

```
fopencookie(cookie, mode, iofuncs):
  1. 校验 mode 首字符: 必须为 'r'/'w'/'a', 否则 errno=EINVAL, return NULL
  2. 分配 CookieFile: Box::new(CookieFile { ... })
     若分配失败: return NULL
  3. 若 mode 不含 '+':
        f.flags = if *mode == 'r' { F_NOWR } else { F_NORD }
  4. 设置 fc:
        f.fc.cookie = cookie
        f.fc.iofuncs = iofuncs
  5. 初始化 FILE:
        f.file.fd = -1
        f.file.cookie = &f.fc as *const _ as *mut c_void
        f.file.buf = &f.buf[UNGET..] 的指针
        f.file.buf_size = sizeof(f.buf) - UNGET
        f.file.lbf = EOF
  6. 设置操作函数指针:
        f.file.read  = cookie_read   (内部封装函数)
        f.file.write = cookie_write  (内部封装函数)
        f.file.seek  = cookie_seek   (内部封装函数)
        f.file.close = cookie_close  (内部封装函数)
  7. return __ofl_add(&f.file)
```

时间复杂度 O(1)。

---

### 内部回调封装函数设计（Rust 安全重构）

以下内部函数在原 C 实现中为 `static` 函数，封装用户回调。在 Rust 侧可重新设计为模块私有函数，将用户裸函数指针调用集中在有限的 `unsafe` 块内。

#### cookie_read (对应 C 的 cookieread)

```rust
/// 封装用户读取回调的 FILE 兼容函数
/// - 若用户 read 回调为 None：设置 F_EOF，返回 0
/// - 若回调返回 0：设置 F_EOF
/// - 若回调返回 < 0：设置 F_ERR
/// - 成功时：返回实际读取字节数（含预读到 FILE 缓冲区的一字节）
fn cookie_read(fc: &FCookie, buf: &mut [u8], len: usize, f_flags: &mut c_int) -> usize;
```

**算法:**
```
cookie_read(fc, buf, len, f_flags):
  // 为预读预留 1 字节（若 buf_size > 0）
  len2 = len.saturating_sub(1)

  if fc.iofuncs.read.is_none():
    *f_flags |= F_EOF
    return 0

  // 先做一次大读取
  if len2 > 0:
    ret = unsafe { (fc.iofuncs.read.unwrap())(fc.cookie, buf.as_mut_ptr() as _, len2) }
    if ret <= 0:
      *f_flags |= if ret == 0 { F_EOF } else { F_ERR }
      return 0
    readlen = ret as usize

    // 若不需要预读，直接返回
    if buf_size == 0 || len - readlen <= 1:
      return readlen

  // 预读 1 字节到 FILE 内部缓冲区
  ret = unsafe { (fc.iofuncs.read.unwrap())(fc.cookie, pread_buf, 1) }
  if ret <= 0:
    *f_flags |= if ret == 0 { F_EOF } else { F_ERR }
    return readlen
  buf[readlen] = pread_buf[0]
  return readlen + 1
```

#### cookie_write (对应 C 的 cookiewrite)

```rust
/// 封装用户写入回调的 FILE 兼容函数
/// - 若用户 write 回调为 None：返回 len（静默丢弃数据）
/// - 若回调返回 < 0：重置写缓冲区，设置 F_ERR，返回 0
fn cookie_write(fc: &FCookie, buf: &[u8], len: usize, f_flags: &mut c_int) -> usize;
```

**算法:**
```
cookie_write(fc, buf, len, f_flags):
  if fc.iofuncs.write.is_none():
    return len  // 无写入回调: 假装写入成功

  ret = unsafe { (fc.iofuncs.write.unwrap())(fc.cookie, buf.as_ptr() as _, len) }
  if ret < 0:
    *f_flags |= F_ERR
    return 0
  return ret as usize
```

#### cookie_seek (对应 C 的 cookieseek)

```rust
/// 封装用户 seek 回调
/// - whence > 2: errno = EINVAL, 返回 -1
/// - 用户 seek 回调为 None: errno = ENOTSUP, 返回 -1
/// - 成功: 返回新偏移量
fn cookie_seek(fc: &FCookie, off: *mut i64, whence: c_int) -> Result<usize, ()>;
```

**算法:**
```
cookie_seek(fc, off, whence):
  if whence > 2: errno = EINVAL, return Err(())
  if fc.iofuncs.seek.is_none(): errno = ENOTSUP, return Err(())
  res = unsafe { (fc.iofuncs.seek.unwrap())(fc.cookie, off, whence) }
  if res < 0: return Err(())
  Ok(unsafe { *off } as usize)
```

#### cookie_close (对应 C 的 cookieclose)

```rust
/// 封装用户关闭回调
/// - 若用户 close 回调为 None：返回 0
/// - 否则调用用户回调并返回其结果
fn cookie_close(fc: &FCookie) -> c_int;
```

---

## 依赖图

```
fopencookie
  ├─> FCookie (struct)          (内部定义)
  ├─> CookieFile (struct)       (内部定义)
  ├─> cookie_read               (内部定义, pub(crate))
  ├─> cookie_write              (内部定义, pub(crate))
  ├─> cookie_seek               (内部定义, pub(crate))
  ├─> cookie_close              (内部定义, pub(crate))
  ├─> __ofl_add                 (see ofl_add.rs spec — 注册到全局打开文件链表)
  ├─> cookie_io_functions_t     (来自 ABI 定义, repr(C))
  └─> alloc 模块               (Box 分配, 替代 C 的 malloc/memset)
```

---

## [RELY]

- `__ofl_add`: 将新 FILE 注册到全局打开文件链表（定义于 `rusl-stdio` 的 `ofl_add` 模块）
- `cookie_io_functions_t`: ABI 兼容的 C 结构体，定义于本模块的对外接口中
- Rust `alloc` 模块：`Box` 用于内存管理，替代 C 的 `malloc`/`memset`

## [GUARANTEE]

Exported Interface:
```
#[repr(C)]
pub struct cookie_io_functions_t { ... };

unsafe extern "C" fn fopencookie(
    cookie: *mut c_void,
    mode: *const c_char,
    iofuncs: cookie_io_functions_t,
) -> *mut FILE;
```

本模块保证对外提供 ABI 兼容的 `fopencookie` 符号及 `cookie_io_functions_t` 结构体。行为符合 GNU `fopencookie()` 扩展语义：创建用户回调驱动的 `FILE` 流，设置正确的读/写/seek/close 封装函数，支持可选的 `NULL` 回调（不支持的操作用适当默认行为处理）。

内部所有封装函数（`cookie_read`、`cookie_write`、`cookie_seek`、`cookie_close`）及结构体（`FCookie`、`CookieFile`）均为模块私有，不对外暴露。
