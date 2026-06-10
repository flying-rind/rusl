# __fdopen 函数规约

## 复杂度分级: Level 3

> musl libc 内部 `fdopen` 主实现。从已打开的文件描述符和 mode 字符串构造 `FILE` 流对象，分配内存、配置缓冲区、设置操作函数指针，并将流登记到全局打开文件链表中。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};

extern "C" fn __fdopen(fd: c_int, mode: *const c_char) -> *mut FILE;

// weak_alias: fdopen 是 __fdopen 的弱别名，共享同一实现
extern "C" fn fdopen(fd: c_int, mode: *const c_char) -> *mut FILE;
```

[Visibility]: `__fdopen` 为 Internal (hidden) — musl 内部实现，不直接对外暴露。通过 `weak_alias(__fdopen, fdopen)` 将 `fdopen` 作为 POSIX 标准函数对外暴露给用户。两个符号均在编译产物中以 `#[no_mangle]` 导出，保证 C 侧链接可见性和 ABI 兼容性。

---

### 前置/后置条件

**[Pre-condition]:**
- `fd`: 有效的已打开文件描述符（`>= 0`）
- `mode`: 非空（non-null）的合法 mode 字符串，以 `'\0'` 结尾，首字符为 `'r'`、`'w'` 或 `'a'`

**[Post-condition]:**

**Case 1: 成功**
- 在堆上分配 `sizeof(FILE) + UNGET + BUFSIZ` 字节的内存（内部可通过 Rust 的 `alloc::alloc` 实现）
- `(*f).fd = fd`
- `(*f).buf` 指向紧接 `FILE` 结构体之后（偏移 `sizeof(FILE) + UNGET`）
- `(*f).buf_size = BUFSIZ`
- 根据 mode 设置了 `(*f).flags`（读写限制、追加等标志）
- `(*f).lbf = '\n'`（若为可写终端）或 `EOF`（否则）
- 所有操作函数指针已设置：
  - `(*f).read = __stdio_read`
  - `(*f).write = __stdio_write`
  - `(*f).seek = __stdio_seek`
  - `(*f).close = __stdio_close`
- `(*f).lock = -1`（若 libc 为单线程模式）
- `f` 已加入全局打开文件链表（通过 `__ofl_add`）
- 返回 `f`（非空指针）

**Case 2: 失败**
- 若 mode 首字符无效（非 `'r'`/`'w'`/`'a'`）：设置 `errno = EINVAL`，返回 `core::ptr::null_mut()`
- 若内存分配失败：返回 `core::ptr::null_mut()`，errno 由分配器设置（通常为 `ENOMEM`）

**[Error Behavior]:**
- mode 首字符非法: `errno = EINVAL`
- 内存分配失败: errno 由底层分配器设置（`ENOMEM`）

---

### 不变量

**[Invariant]:**
- `(*f).buf` 总位于 `(unsigned char*)f + sizeof(*f) + UNGET`，即 FILE 结构体后 `UNGET` 字节开始
- `UNGET`（8 字节）为 `ungetc` 保留空间，位于 FILE 结构体和写缓冲区之间
- `(*f).lock = -1` 仅在单线程模式（`!libc.threaded`）时设置；多线程模式下 `(*f).lock = 0`（由 `memset` 确保）
- `(*f).read`/`(*f).write`/`(*f).seek`/`(*f).close` 始终被设置为默认 stdio 操作函数
- `fdopen` 与 `__fdopen` 共享同一函数体，行为完全一致

---

### 意图

将已有的文件描述符封装为 `FILE` 流，根据 mode 字符串配置流的读写权限、缓冲策略和操作函数指针。

Rust 侧实现：
- `FILE` 结构体定义为 `#[repr(C)]` 以保持 ABI 兼容
- 内部使用 `alloc::alloc::alloc` / `alloc::alloc::alloc_zeroed` 进行堆内存分配，替代 C 的 `malloc`/`memset`
- mode 字符串解析使用安全的字符串操作（如遍历 `&[u8]`，而非 `strchr` 裸指针操作）
- 终端检测（`ioctl(TIOCGWINSZ)`）通过 `syscall!` 宏或内联汇编实现
- `__ofl_add` 通过内部链表管理模块调用
- 提供 `fdopen` 作为弱别名：在 Rust 侧通过复制函数体或 `#[link_name]`/linker 脚本实现真正的弱符号别名

---

### 系统算法

```
__fdopen(fd, mode):
  /* 1. 验证 mode 首字符 */
  if *mode not in {'r', 'w', 'a'}:
    errno = EINVAL
    return core::ptr::null_mut()

  /* 2. 分配 FILE + 缓冲区 */
  f = alloc(sizeof(FILE) + UNGET + BUFSIZ)
  if f.is_null():
    return core::ptr::null_mut()

  /* 3. 仅清零结构体，不清零缓冲区（UNGET + buf 区域） */
  memset_zero(f, sizeof(FILE))

  /* 4. 读写限制 */
  if '+' not in mode:
    f.flags = if *mode == 'r' { F_NOWR } else { F_NORD }

  /* 5. close-on-exec */
  if 'e' in mode:
    syscall!(SYS_fcntl, fd, F_SETFD, FD_CLOEXEC)

  /* 6. 追加模式 */
  if *mode == 'a':
    flags = syscall!(SYS_fcntl, fd, F_GETFL)
    if flags & O_APPEND == 0:
      syscall!(SYS_fcntl, fd, F_SETFL, flags | O_APPEND)
    f.flags |= F_APP

  /* 7. 设置 fd 和缓冲区 */
  f.fd = fd
  f.buf = f as *mut u8 + sizeof(FILE) + UNGET
  f.buf_size = BUFSIZ

  /* 8. 终端检测 -> 行缓冲 */
  f.lbf = EOF
  if (f.flags & F_NOWR) == 0 && syscall!(SYS_ioctl, fd, TIOCGWINSZ, &wsz) == 0:
    f.lbf = '\n' as c_int

  /* 9. 设置操作函数指针 */
  f.read = __stdio_read
  f.write = __stdio_write
  f.seek = __stdio_seek
  f.close = __stdio_close

  /* 10. 单线程模式：预设 lock = -1 */
  if !libc_state.threaded:
    f.lock = -1

  /* 11. 加入全局打开文件链表 */
  return __ofl_add(f)
```

时间复杂度 O(1)（不含系统调用开销）。

---

## 依赖图

```
__fdopen / fdopen
  ├─> core::ptr          (指针操作)
  ├─> alloc::alloc       (堆内存分配)
  ├─> syscall!(SYS_fcntl)  (内核)
  ├─> syscall!(SYS_ioctl)  (内核)
  ├─> __stdio_read       (see __stdio_read spec)
  ├─> __stdio_write      (see __stdio_write spec)
  ├─> __stdio_seek       (see __stdio_seek spec)
  ├─> __stdio_close      (see __stdio_close spec)
  ├─> __ofl_add          (see ofl_add spec)
  └─> libc_state         (libc 全局运行时状态)
```

---

## [RELY]

- `alloc::alloc::alloc` — 堆内存分配（Rust 核心分配器）
- `syscall!` 宏 — 系统调用接口（`SYS_fcntl`、`SYS_ioctl`）
- `__stdio_read` / `__stdio_write` / `__stdio_seek` / `__stdio_close` — 默认流操作函数（本模块）
- `__ofl_add` — 全局打开文件链表注册（本模块）
- `libc_state` — 全局运行时状态（线程模式标志）
- 常量: `UNGET`, `BUFSIZ`, `F_NOWR`, `F_NORD`, `F_APP`, `EINVAL`, `EOF`, `O_APPEND`, `FD_CLOEXEC`, `F_SETFD`, `F_SETFL`, `F_GETFL`, `TIOCGWINSZ`

## [GUARANTEE]

Exported Interface:
  `extern "C" fn __fdopen(fd: c_int, mode: *const c_char) -> *mut FILE;`
  `extern "C" fn fdopen(fd: c_int, mode: *const c_char) -> *mut FILE;`

本模块保证对外提供上述两个 ABI 兼容的函数符号，行为符合 POSIX `fdopen` 语义。`fdopen` 与 `__fdopen` 行为完全一致，为弱别名关系。
