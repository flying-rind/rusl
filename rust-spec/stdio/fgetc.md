# fgetc 函数规约

## 复杂度分级: Level 1

> musl libc 标准 IO 单字符读取实现。从 FILE 流中读取一个字符。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uchar};

// FILE 为 opaque 类型，定义于 rusl-internal 模块
// use rusl_internal::FILE;

extern "C" fn fgetc(f: *mut FILE) -> c_int;
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用。必须保持 ABI 兼容：`extern "C"` 导出，参数类型布局与原 C 接口一致。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: 非空 `*mut FILE` 指针，指向已打开的读模式流。
- 流中存在可读数据（或已到达 EOF）。

**[Post-condition]:**
- **Case 1 成功读取字符**
  - 返回读取到的字符（以 `c_uchar` 转换的 `c_int`，范围 0-255）。
  - FILE 流位置前进一个字符。

- **Case 2 到达文件末尾**
  - 返回 `EOF`（通常为 -1）。
  - FILE 流设置 `F_EOF` 标志。

- **Case 3 读取错误**
  - 返回 `EOF`。
  - FILE 流设置 `F_ERR` 标志。
  - `errno` 可能被设置（由底层 `read` 系统调用设置）。

**[Error Behavior]:**
- 到达 EOF 或读取错误时返回 `EOF`。调用者需通过 `feof(f)` / `ferror(f)` 区分原因。

---

### 不变量

**[Invariant]:**
- 成功读取时返回值始终在 0-255 范围内。
- `f` 指向的 FILE 结构在函数返回后保持有效状态。

---

### 意图

从 FILE 流 `f` 中读取一个 `unsigned char` 类型的字符，以 `int` 返回。若到达文件末尾或发生读取错误，返回 `EOF`。

Rust 侧实现：
- 内部通过 `do_getc(f)` 完成实际读取，`do_getc` 定义于 `getc.h` 对应的 Rust 内部模块中。
- `do_getc` 负责智能锁管理（根据流是否为线程安全模式决定是否加锁）和字符读取逻辑。
- `fgetc` 函数体直接委托给 `do_getc(f)`。

### 系统算法

```
fgetc(f: *mut FILE) -> c_int:
  return do_getc(f)
```

参见 `getc.h` 对应的 Rust spec 中 `do_getc` 的完整算法描述。

时间复杂度 O(1)（不含底层 I/O）。

---

## 依赖图

```
fgetc (Public)
  └── do_getc (内部 inline 函数, from "getc.rs" 内部模块)
        ├── getc_unlocked (内部宏/内联函数)
        │     └── __uflow (see __uflow spec)
        ├── locking_getc (内部 static 函数, from "getc.rs" 内部模块)
        │     ├── __lockfile / atomic CAS / __wake
        │     └── getc_unlocked
        └── __pthread_self
```

---

## [RELY]

- `do_getc(FILE)` — 内部字符读取函数（`getc.rs` 内部模块）。
- `__uflow(FILE)` — 底层缓冲区填充函数。
- `__lockfile(FILE)` / `__unlockfile(FILE)` — 文件锁函数。
- `__pthread_self()` — 线程自引用（用于免锁优化）。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn fgetc(f: *mut FILE) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为符合 ISO C 标准 `fgetc` 语义。
