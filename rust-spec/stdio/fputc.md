# fputc 函数规约

## 复杂度分级: Level 1

> musl libc 标准 IO 单字符写入实现。将一个字符写入 FILE 流。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为 opaque 类型，定义于 rusl-internal 模块

extern "C" fn fputc(c: c_int, f: *mut FILE) -> c_int;
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用。必须保持 ABI 兼容：`extern "C"` 导出，参数类型布局与原 C 接口一致。

---

### 前置/后置条件

**[Pre-condition]:**
- `c`: 要写入的字符（以 `c_int` 传递，高位字节被忽略，仅低 8 位有效）。
- `f`: 非空 `*mut FILE` 指针，指向已打开的写模式流。

**[Post-condition]:**
- **Case 1 成功写入**
  - 返回写入的字符（以 `unsigned char` 转换的 `c_int`，范围 0-255）。
  - 字符已写入流缓冲区或通过底层 write 写入。

- **Case 2 写入错误**
  - 返回 `EOF`（通常为 -1）。
  - FILE 流设置 `F_ERR` 标志。
  - `errno` 可能被设置。

**[Error Behavior]:**
- 写入错误时返回 `EOF`。调用者需通过 `ferror(f)` 确认。

---

### 不变量

**[Invariant]:**
- 对于行缓冲流，遇到 `\n` 时缓冲区会被刷出。
- 成功写入时返回值始终在 0-255 范围内。

---

### 意图

将字符 `c`（转换为 `unsigned char`）写入 FILE 流 `f`。内部通过 `do_putc` 实现智能锁管理。

Rust 侧实现：
- 内部通过 `do_putc(c, f)` 完成实际写入，`do_putc` 定义于 `putc.h` 对应的 Rust 内部模块中。
- `do_putc` 负责智能锁管理（根据流是否为线程安全模式决定是否加锁）和字符写入逻辑。
- `fputc` 函数体直接委托给 `do_putc(c, f)`。

### 系统算法

```
fputc(c: c_int, f: *mut FILE) -> c_int:
  return do_putc(c, f)
```

参见 `putc.h` 对应的 Rust spec 中 `do_putc` 的完整算法描述。

时间复杂度 O(1)（不含底层 I/O）。

---

## 依赖图

```
fputc (Public)
  └── do_putc (内部 inline 函数, from "putc.rs" 内部模块)
        ├── putc_unlocked (内部宏/内联函数)
        │     └── __overflow (see __overflow spec)
        ├── locking_putc (内部 static 函数, from "putc.rs" 内部模块)
        │     ├── __lockfile / atomic CAS / __wake
        │     └── putc_unlocked
        └── __pthread_self
```

---

## [RELY]

- `do_putc(int, FILE)` — 内部字符写入函数（`putc.rs` 内部模块）。
- `__overflow(FILE)` — 底层缓冲区刷出函数。
- `__lockfile(FILE)` / `__unlockfile(FILE)` — 文件锁函数。
- `__pthread_self()` — 线程自引用（用于免锁优化）。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn fputc(c: c_int, f: *mut FILE) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为符合 ISO C 标准 `fputc` 语义。
