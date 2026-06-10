# ungetc 函数规约

## 复杂度分级: Level 2

> musl libc `ungetc` 实现 — 将一个字符推回 FILE 流的输入缓冲区。这是 ISO C 标准中单字节字符回退的标准接口。

---

## 函数接口

```rust
use core::ffi::c_int;
use core::ffi::c_uint; // 用于 unsigned char 转换

extern "C" fn ungetc(c: c_int, f: *mut FILE) -> c_int;
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用。在 Rust 侧通过 `#[no_mangle] pub unsafe extern "C"` 导出，保持 ABI 兼容。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: 有效的 `*mut FILE` 指针，指向已打开的流（非空）
- `c`: 要推回的字符值（以 `c_int` 传递，仅低 8 位有效）
- 流 `f` 必须处于读模式或有足够空间容纳推回的字符

**[Post-condition]:**
- Case 1 成功推回:
  - `c` 的值（转换为 `u8`）被推回读缓冲区（`(*f).rpos` 递减并写入）
  - 流的 `F_EOF` 标志被清除
  - 返回转换为 `c_int` 的 `c as u8 as c_int` 值

- Case 2 输入为 EOF:
  - 流状态不变
  - 返回 `EOF`

- Case 3 推回失败（流未处于读模式或缓冲区空间不足）:
  - 流状态不变
  - 返回 `EOF`

**[Error Behavior]:**
- 推回失败时返回 `EOF`（与 `ungetc(EOF, f)` 返回 `EOF` 不可区分，符合 POSIX 定义）
- 不设置 errno（标准未要求）

---

### 不变量

**[Invariant]:**
- 推回区域始终在缓冲区真实数据之前，`rpos` 可安全回退至 `(*f).buf.offset(-(UNGET as isize))`
- 成功推回至少一个字符总是可行的，前提是流处于读模式且 `rpos` 有效
- 推回不改变流的错误状态（仅清除 EOF 标志）
- 函数执行过程中获取并释放 FILE 锁，保证线程安全

---

### 意图

将字符 `c` 推回 FILE 流 `f` 的读缓冲区，使得下一次从流中读取时返回该字符。推回的字符被转换为 `unsigned char` 后存放在读缓冲区中。

该函数是标准 C 中唯一的字符回退接口。保证至少可以成功推回一个字符。成功推回后，流的 EOF 状态被清除。

Rust 侧实现：
- `FILE` 类型为 musl 内部 `struct _IO_FILE` 的 Rust 表示（`#[repr(C)]`），通过 `*mut FILE` 裸指针传递
- 内部需要调用 `__toread` 初始化读模式，以及 `__lockfile`/`__unlockfile` 进行流加锁/解锁
- 字符写入使用 `ptr::write` 或直接通过解引用裸指针写入
- `UNGET` 和 `F_EOF` 定义为模块级常量
- 锁操作可通过 `FLOCK`/`FUNLOCK` 宏等价的内联函数实现，仅在 `(*f).lock >= 0` 时加锁

### 系统算法

```
ungetc(c, f):
  1. 若 c == EOF，返回 c（EOF 不可推回）
  2. FLOCK(f) — 获取流锁
  3. 若 (*f).rpos 为 null，调用 __toread(f) 初始化读模式
  4. 若 (*f).rpos 仍为 null（无法进入读模式），或
     (*f).rpos <= (*f).buf.offset(-(UNGET as isize))（推回超出预留空间）:
       释放锁，返回 EOF
  5. (*f).rpos = (*f).rpos.offset(-1)  — 递减 rpos
     *(*f).rpos = c as u8                 — 写入字符
  6. (*f).flags &= !F_EOF — 清除 EOF 标志
  7. FUNLOCK(f) — 释放锁
  8. 返回 c as u8 as c_int
```

时间复杂度 O(1)（不含 `__toread` 的潜在 O(n) 模式切换）。

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `EOF` | (-1) | `<stdio.h>` | 文件结束标志，不可推回 |
| `UNGET` | 8 | `stdio_impl.h` | 字符回退预留空间大小 |
| `F_EOF` | 16 | `stdio_impl.h` | 流 EOF 状态标志位 |

---

## 依赖图

```
ungetc (Public)
  ├── FLOCK / FUNLOCK (锁宏, 定义于 stdio_impl.h)
  │     ├── __lockfile (see __lockfile spec)
  │     └── __unlockfile (see __unlockfile spec)
  └── __toread (see __toread spec)
```

---

## [RELY]

- `__toread(f: *mut FILE)` — 将流切换到读模式（见 `__toread` spec）
- `__lockfile(f: *mut FILE)` / `__unlockfile(f: *mut FILE)` — 流加锁/解锁（见 `__lockfile` spec）
- `FLOCK` / `FUNLOCK` 宏 — 仅在 `(*f).lock >= 0` 时加锁/解锁
- `FILE` 结构体定义（含 `rpos`、`buf`、`flags`、`lock` 字段）
- 常量 `EOF`、`UNGET`、`F_EOF`

## [GUARANTEE]

Exported Interface:
  `extern "C" fn ungetc(c: c_int, f: *mut FILE) -> c_int;`

本模块保证对外提供 ABI 兼容的 `ungetc` 函数符号，行为符合 POSIX/C11 标准定义。保证至少可以成功推回一个字符（前提是流处于读模式且 `rpos` 有效），成功推回后流的 EOF 状态被清除。
