# __fopen_rb_ca 函数规约

## 复杂度分级: Level 2

> musl libc 内部调用方分配 FILE（Caller-Allocated）的只读打开实现。由调用方提供 `FILE` 结构体内存和缓冲区，以只读方式打开文件，并设置必要的流操作函数指针。用于实现 `freopen` 等需要复用 `FILE` 内存的场景。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int, c_ulong};

extern "C" fn __fopen_rb_ca(
    filename: *const c_char,
    f: *mut FILE,
    buf: *mut u8,
    len: usize,
) -> *mut FILE;
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。与 `__fclose_ca` 配套，供 `freopen` 等复用 `FILE` 内存的内部实现调用。

---

### 前置/后置条件

**[Pre-condition]:**
- `filename`: 非空（non-null）的文件路径字符串，以 `'\0'` 结尾
- `f`: `*mut FILE`，调用方提供的 `FILE` 内存（有效地址，可写）
- `buf`: `*mut u8`，调用方提供的缓冲区，长度至少为 `len`
- `len`: 缓冲区总长度，必须大于 `UNGET`（通常为 `BUFSIZ + UNGET`）

**[Post-condition]:**

**Case 1: 成功打开文件**
- `(*f)` 的全部字段首先被清零
- `(*f).fd` = 打开的文件描述符（`>= 0`，`O_RDONLY | O_CLOEXEC`）
- `(*f).fd` 的 `FD_CLOEXEC` 已通过 `fcntl(F_SETFD)` 确认设置
- `(*f).flags = F_NOWR | F_PERM`（禁止写 + 永久流标志）
- `(*f).buf = buf + UNGET`（为 ungetc 预留 `UNGET` 字节）
- `(*f).buf_size = len - UNGET`（可用缓冲区大小）
- `(*f).read = __stdio_read`
- `(*f).seek = __stdio_seek`
- `(*f).close = __stdio_close`
- `(*f).lock = -1`（初始无锁，单线程模式）
- 返回 `f`（指向调用方提供的 `FILE`）

**Case 2: 打开文件失败**
- `(*f)` 已被清零（字段被覆盖但有效）
- `(*f).fd = -1`（由 `sys_open` 设置的错误返回值）
- 返回 `core::ptr::null_mut()`
- errno 由底层系统调用设置

**[Error Behavior]:**
- 打开文件失败：返回 null，errno 由 `sys_open` 系统调用设置（如 `ENOENT`、`EACCES` 等）

---

### 不变量

**[Invariant]:**
- `(*f).flags` 始终包含 `F_NOWR`（禁止写入）
- `(*f).buf` 始终为 `buf + UNGET`（内部缓冲区前预留反推空间）
- `(*f).lock = -1` 表示初始无锁，`FLOCK` 宏将此视为无需加锁
- 不设置 `(*f).write` 函数指针（读打开流不支持写操作，调用写入将导致未定义行为）

---

### 意图

以只读方式打开文件，使用调用方提供的 `FILE` 内存和缓冲区。文件以 `O_RDONLY | O_CLOEXEC` 打开，并设置 close-on-exec 标志（双重保险）。为 `f` 设置只读操作函数指针（`read`、`seek`、`close`），不设置 `write` 函数指针。

Rust 侧实现：
- 清零 `FILE` 结构体使用 `core::ptr::write_bytes` 或 `core::slice::from_raw_parts_mut` + 字节循环
- 打开文件使用 `syscall!` 宏（`SYS_openat` 或 `SYS_open`）
- `f->buf` 偏移计算使用 `pointer::offset` 或 `pointer::wrapping_add`
- `FILE` 结构体定义为 `#[repr(C)]` 以保持 ABI 兼容
- 内部可将 `FILE` 的初始化逻辑封装为安全的 builder 模式，但在 `extern "C"` 边界仍需传递裸指针

---

### 系统算法

```
__fopen_rb_ca(filename, f, buf, len):
  /* 1. 清零 FILE 结构体 */
  zero_memory(f, size_of::<FILE>())

  /* 2. 以只读+close-on-exec 打开文件 */
  fd = syscall!(SYS_openat, AT_FDCWD, filename, O_RDONLY | O_CLOEXEC, 0)
  if fd < 0:
    return core::ptr::null_mut()
  (*f).fd = fd

  /* 3. 双重确认 close-on-exec */
  syscall!(SYS_fcntl, (*f).fd, F_SETFD, FD_CLOEXEC)

  /* 4. 设置流标志与缓冲区 */
  (*f).flags = F_NOWR | F_PERM
  (*f).buf = buf.add(UNGET)          // 预留 UNGET 字节
  (*f).buf_size = len - UNGET

  /* 5. 设置操作函数指针（只读，不设 write） */
  (*f).read = __stdio_read
  (*f).seek = __stdio_seek
  (*f).close = __stdio_close
  (*f).lock = -1                     // 单线程模式

  return f
```

时间复杂度 O(1)（不含系统调用开销）。

---

## 依赖图

```
__fopen_rb_ca
  ├─> core::ptr          (裸指针操作 / 内存清零)
  ├─> syscall!(SYS_openat)   (内核)
  ├─> syscall!(SYS_fcntl)    (内核)
  ├─> __stdio_read       (see __stdio_read spec)
  ├─> __stdio_seek       (see __stdio_seek spec)
  └─> __stdio_close      (see __stdio_close spec)
```

---

## [RELY]

- `core::ptr` — 裸指针操作和内存清零
- `syscall!` 宏 — 系统调用接口（`SYS_openat`、`SYS_fcntl`）
- `__stdio_read` / `__stdio_seek` / `__stdio_close` — 默认流操作函数（本模块）
- 常量: `UNGET`, `F_NOWR`, `F_PERM`, `O_RDONLY`, `O_CLOEXEC`, `FD_CLOEXEC`, `F_SETFD`, `AT_FDCWD`

## [GUARANTEE]

Exported Interface:
  `extern "C" fn __fopen_rb_ca(filename: *const c_char, f: *mut FILE, buf: *mut u8, len: usize) -> *mut FILE;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为和原 C 实现完全一致。
