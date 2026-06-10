# __stdio_seek 函数规约

## 复杂度分级: Level 1

> musl libc 内部 FILE 默认定位操作实现。作为 `f->seek` 函数指针的默认值，将定位请求直接转发给 `__lseek` 系统调用。

---

## 函数接口

```rust
use core::ffi::c_int;

// off_t 在 64 位 Linux 上为 i64，在 Rust 中使用 i64 或专门的 off_t 类型
extern "C" fn __stdio_seek(f: *mut FILE, off: i64, whence: c_int) -> i64;
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。作为 `f->seek` 函数指针的默认值，被 `fseek`、`fseeko` 等间接调用。

> 注意：`off_t` 在 64 位 musl 上为 `long`（8 字节），对应 Rust 的 `i64`。`whence` 是 `c_int`。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: `*mut FILE`，非空指针，其 `fd` 为有效的文件描述符（支持定位操作，如普通文件）
- `off`: 偏移量（以字节为单位，可为负值配合 `SEEK_CUR`/`SEEK_END`）
- `whence`: 定位基准，合法值：
  - `SEEK_SET` (0)：文件起始
  - `SEEK_CUR` (1)：当前位置
  - `SEEK_END` (2)：文件末尾

**[Post-condition]:**

**Case 1: 成功**
- 文件偏移量被更新到新位置
- 返回新的文件偏移量（从文件起始的字节偏移，`>= 0`）

**Case 2: 失败**
- 返回 `-1`（即 `(off_t)-1` / `-1_i64`）
- 设置 errno（由 `__lseek` 设置，如 `EBADF`、`EINVAL`、`ESPIPE` 等）

**[Error Behavior]:**
- 本函数不自行设置 errno，错误由底层 `lseek` 系统调用设置

---

### 不变量

**[Invariant]:**
- 薄封装（thin wrapper）：仅转发调用，不修改 `f` 的任何字段
- 不访问 `FILE` 结构体中除 `fd` 外的其他字段

---

### 意图

对文件描述符执行定位操作。这是一个薄封装，直接将 `seek` 操作转发给底层的 `__lseek` 系统调用。

Rust 侧实现：
- 调用 `__lseek(f->fd, off, whence)`，`__lseek` 定义为另一个 `extern "C"` 函数或通过 `syscall!` 直接内联
- 函数签名保持 `extern "C"` 以兼容 `f->seek` 函数指针类型
- 内部可考虑对 `whence` 值进行 debug 断言验证（合法值 0/1/2）
- 由于仅是转发调用，可考虑直接内联 `syscall!(SYS_lseek, ...)` 以避免额外的函数调用开销

---

### 系统算法

```
__stdio_seek(f, off, whence):
  return __lseek((*f).fd, off, whence)
```

时间复杂度 O(1)（不含系统调用开销）。

---

## 依赖图

```
__stdio_seek
  └─> __lseek / syscall!(SYS_lseek)   (内核)
```

---

## [RELY]

- `__lseek` — 文件定位系统调用封装（`unistd` 模块，或直接通过 `syscall!` 宏调用 `SYS_lseek`）
- 常量: `SEEK_SET` (0), `SEEK_CUR` (1), `SEEK_END` (2)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn __stdio_seek(f: *mut FILE, off: i64, whence: c_int) -> i64;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为与原 C 实现完全一致：直接转发 `lseek` 调用，返回新的文件偏移量或 `-1`（失败时）。
