# setvbuf 函数规约

## 复杂度分级: Level 1

> musl libc `setvbuf` 实现 — 所有缓冲设置函数的最终实现。直接操作 FILE 结构体的内部字段，不执行加锁（因为标准规定 setvbuf 仅在流打开后首次操作前调用时行为才定义良好）。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};
use core::ffi::c_uint; // size_t

extern "C" fn setvbuf(f: *mut FILE, buf: *mut c_char, type_: c_int, size: size_t) -> c_int;
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用。在 Rust 侧通过 `#[no_mangle] pub unsafe extern "C"` 导出，保持 ABI 兼容。

---

### 内部类型 / 结构体字段

`setvbuf` 直接访问 `FILE` 结构体中的以下字段：

| 字段 | Rust 类型 | 用途 |
|------|----------|------|
| `f.lbf` | `c_int` | 行缓冲标志。`EOF`（-1）= 全缓冲/无缓冲；`b'\n'` = 行缓冲 |
| `f.buf` | `*mut u8` | 流内部缓冲区指针（偏移 UNGET 后的实际起始位置） |
| `f.buf_size` | `usize` | 有效缓冲区大小（已扣除 UNGET 预留空间） |
| `f.flags` | `c_uint` | 文件流状态标志位集 |

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: 有效的 `*mut FILE` 指针，指向已打开的流（非空）
- `buf`: 调用方提供的缓冲区指针（可为 null）
- `type_`: 缓冲类型，必须为以下值之一：
  - `_IOFBF` (0): 全缓冲
  - `_IOLBF` (1): 行缓冲
  - `_IONBF` (2): 无缓冲
- `size`: 缓冲区大小（字节）。当 `type_ == _IONBF` 或 `buf` 为 null 时被忽略
- **C 标准要求**：该函数应当仅在流被打开后、任何其他操作（包括成功的 `setvbuf` 调用）之前调用，否则行为未定义

**[Post-condition]:**
- Case 1 成功（`type_` 为 `_IONBF`）:
  - `f.lbf` 设为 `EOF`（-1）
  - `f.buf_size` 设为 0（无缓冲）
  - `F_SVB` 标志置位
  - 返回 0

- Case 2 成功（`type_` 为 `_IOFBF` 或 `_IOLBF`，且提供了足够大的缓冲区）:
  - `f.lbf` 设为 `EOF`（-1）
  - `f.buf` 设为 `buf.offset(UNGET)`，即实际缓冲区起始位置
  - `f.buf_size` 设为 `size - UNGET`
  - 若 `type_ == _IOLBF` 且 `f.buf_size > 0`，`f.lbf` 进一步设为 `b'\n'`
  - `F_SVB` 标志置位
  - 返回 0

- Case 3 成功（`type_` 为 `_IOFBF` 或 `_IOLBF`，但未提供足够大缓冲区或 `buf` 为 null）:
  - `f.lbf` 设为 `EOF`（-1）
  - `f.buf` 和 `f.buf_size` 保持不变（不设置）
  - `F_SVB` 标志置位
  - 返回 0

- Case 4 失败（`type_` 为无效值）:
  - 返回 -1，不修改流状态

**[Error Behavior]:**
- 仅当 `type_` 不是 `_IOFBF`、`_IOLBF`、`_IONBF` 中任一个时返回 -1
- 不设置 errno

---

### 不变量

**[Invariant]:**
- `F_SVB` 标志置位后，表明流的缓冲区已由 `setvbuf` 系列函数设置，后续操作不会重新分配内部缓冲区
- `f.buf - UNGET` 之前的 `UNGET` 字节为字符回退预留区，`rpos` 可回退至 `f.buf - UNGET`
- 函数执行过程中不获取/释放 FILE 锁（无锁设计）

---

### 意图

设置 FILE 流的缓冲模式、缓冲区位置和大小。这是 musl 中 `setbuf`、`setbuffer`、`setlinebuf` 的底层实现。

**关键设计决策**：不执行任何 FILE 锁操作。理由由源码注释给出——该函数的行为由标准定义为仅在流上首次操作前调用才可预测，因此在一个行为定义良好的程序中，加锁的存在与否是不可观察的。

**缓冲区布局**：若调用方提供了缓冲区且 `size >= UNGET`（8），musl 在内部分配 `UNGET` 字节预留空间用于字符回退操作。实际缓冲区起始于 `buf + UNGET`，有效大小为 `size - UNGET`。若 `size < UNGET`，则不使用调用方提供的缓冲区，此时 `f.buf` 和 `f.buf_size` 保持不变。

Rust 侧实现：
- 使用 `core::ffi::c_int`、`core::ffi::c_char`、`usize`（对应 C 的 `size_t`）
- `FILE` 类型为 musl 内部 `struct _IO_FILE` 的 Rust 表示，通过 `#[repr(C)]` 保证内存布局与 C 一致
- 缓冲区指针使用 `*mut u8` 而非 `*mut c_char`，避免符号扩展问题
- 内部可定义常量 `UNGET: usize = 8`、`F_SVB: c_uint = 64`、`EOF: c_int = -1` 等

### 系统算法

```
setvbuf(f, buf, type_, size):
  1. f.lbf = EOF
  2. 若 type_ == _IONBF:
       f.buf_size = 0
  3. 否则若 type_ == _IOLBF 或 type_ == _IOFBF:
       a. 若 buf 非 null 且 size >= UNGET (8):
            f.buf = buf.offset(UNGET)  // 等效于 buf + UNGET
            f.buf_size = size - UNGET
       b. 若 type_ == _IOLBF 且 f.buf_size > 0:
            f.lbf = b'\n' as c_int
  4. 否则 (type_ 无效):
       返回 -1
  5. f.flags |= F_SVB
  6. 返回 0
```

时间复杂度 O(1)。

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `_IONBF` | 2 | `<stdio.h>` | 无缓冲模式 |
| `_IOLBF` | 1 | `<stdio.h>` | 行缓冲模式 |
| `_IOFBF` | 0 | `<stdio.h>` | 全缓冲模式 |
| `UNGET` | 8 | `stdio_impl.h` | 回退字符预留空间大小 |
| `F_SVB` | 64 | `stdio_impl.h` | setvbuf 已调用标志位 |
| `EOF` | (-1) | `<stdio.h>` | 文件结束标志（也用作 lbf 中表示非行缓冲的哨兵值） |

---

## 依赖图

```
setvbuf (Public)
  └── (无内部调用 — 仅直接操作 FILE 结构体字段)
```

---

## [RELY]

无内部函数依赖。依赖外部模块提供的 `FILE` 结构体定义（含 `lbf`、`buf`、`buf_size`、`flags` 字段）以及常量定义 `UNGET`、`F_SVB`、`_IONBF`、`_IOLBF`、`_IOFBF`、`EOF`。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn setvbuf(f: *mut FILE, buf: *mut c_char, type_: c_int, size: size_t) -> c_int;`

本模块保证对外提供 ABI 兼容的 `setvbuf` 函数符号，行为符合 POSIX/C11 标准定义。函数不获取/释放 FILE 锁，调用方负责确保在定义良好的程序状态下调用。
