# puts 函数规约

## 复杂度分级: Level 1

> musl libc 标准库字符串输出（带换行）函数。向 stdout 输出字符串并自动追加换行符。Rust 实现中，外部接口保持 ABI 兼容，内部 I/O 调用使用 Rust 安全抽象。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};

unsafe extern "C" fn puts(s: *const c_char) -> c_int;
```

[Visibility]: User — `<stdio.h>` 标准库函数。必须保持 ABI 兼容。`s` 为以 NUL 结尾的 C 字符串指针。返回非负值（通常为 `0`）表示成功，`EOF`（`-1`）表示失败。

---

### 前置/后置条件

**[Pre-condition]:**
- `s`: 非空指针，指向以 NUL 结尾的有效 C 字符串。
- `stdout` 已初始化且可写。
- 调用线程可以获取 `stdout` 的锁。

**[Post-condition]:**
- **Case 1 成功**
  - `s` 的内容后跟一个 `\n` 已写入 `stdout`。
  - 返回非负值（musl 实现中返回 `0`）。

- **Case 2 输出失败**
  - 返回 `EOF`（`-1`）。
  - `stdout` 出错标志可能被设置。

**[Error Behavior]:**
- 写入 `s` 或换行符失败时返回 `-1`（`EOF`）。`stdout` 的 `F_ERR` 标志可能被设置。

---

### 不变量

**[Invariant]:**
- `puts` 始终向 `stdout` 输出，不向其他流。
- 输出始终以 `\n` 结束（无论输入字符串是否以 `\n` 结尾）。
- `puts` 自身持有 `stdout` 锁（加锁/解锁）。

---

### 意图

将 C 字符串 `s`（不含结尾 NUL）写入 `stdout`，随后自动写入一个换行符 `\n`。与 `fputs` 的区别：自动追加换行，始终写入 `stdout`。

Rust 侧实现：
- 外部接口使用 `unsafe extern "C" fn puts(s: *const c_char) -> c_int`，保持 ABI 兼容。
- 内部将 C 字符串指针安全转换为字节切片（通过 `core::ffi::CStr` 或手动遍历）。
- `FLOCK`/`FUNLOCK` 使用 Rust RAII 守卫管理锁，确保异常安全。
- `fputs` 和 `putc_unlocked` 调用内部安全 I/O 函数。
- 返回值逻辑：成功返回 `0`，任一 I/O 操作失败返回 `-1`。Rust 中用 `if` 或布尔短路运算实现同等逻辑。

### 系统算法

```
puts(s):
  1. 锁定 stdout

  2. 写入字符串 s 到 stdout (fputs)
     若失败: 解锁; 返回 -1

  3. 写入换行符 '\n' 到 stdout (putc_unlocked)
     若失败: 解锁; 返回 -1

  4. 解锁 stdout
  5. 返回 0 (成功)
```

时间复杂度 O(strlen(s))。

---

## 依赖图

```
puts (Public, extern "C")
  ├── core::ffi::{c_char, c_int}      — Rust 内置 FFI 类型
  ├── [Internal] fputs                 — 内部安全字符串写入函数
  ├── [Internal] putc_unlocked         — 内部无锁单字符写入函数
  ├── [Internal] stdout                — 标准输出流全局变量
  ├── [Internal] FLOCK / FUNLOCK       — FILE 锁操作
  └── [Internal] EOF                   — 错误返回值常量
```

---

## [RELY]

- `core::ffi::{c_char, c_int}` — Rust 核心库 FFI 类型。
- 内部 `fputs` — rusl 内部安全字符串写入。
- 内部 `putc_unlocked` — rusl 内部无锁单字符写入。
- 内部 `stdout` — rusl 内部标准输出流。
- 内部 FILE 锁定机制 — rusl 内部实现。

## [GUARANTEE]

Exported Interface:
  `unsafe extern "C" fn puts(s: *const c_char) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号：
- 参数 `*const c_char` 与 C `const char *` 内存布局一致。
- 返回值 `c_int` 与 C `int` 完全一致。
- 使用 C 调用约定 (`extern "C"`)。
- 行为符合 C89/C99 `puts()` 语义：向 stdout 写入字符串后自动追加 `\n`，成功返回非负值，失败返回 `EOF`。
