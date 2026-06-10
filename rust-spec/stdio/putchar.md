# putchar 函数规约

## 复杂度分级: Level 1

> musl libc 标准输出单字符写入实现。将一个字符写入 `stdout`。

---

## 函数接口

```rust
use core::ffi::c_int;

// stdout 为全局变量，来自 stdio 模块
// use rusl_internal::stdout;

extern "C" fn putchar(c: c_int) -> c_int;
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用。必须保持 ABI 兼容：`extern "C"` 导出，参数和返回值均为 `c_int`。

---

### 前置/后置条件

**[Pre-condition]:**
- `c`: 要写入的字符（以 `c_int` 传递，内部转为 `unsigned char`）。
- `stdout` 已正确初始化并处于可写状态。

**[Post-condition]:**
- **Case 1 成功写入**
  - 返回写入的字符（0-255 的 `c_int` 值）。
  - 字符已写入 `stdout`。

- **Case 2 写入错误**
  - 返回 `EOF`（-1）。
  - `stdout` 设置 `F_ERR` 标志。

**[Error Behavior]:**
- 写入错误时返回 `EOF`。

---

### 不变量

**[Invariant]:**
- 等价于 `putc(c, stdout)` 或 `fputc(c, stdout)`。
- 若 `stdout` 为行缓冲模式，写入 `\n` 时会触发缓冲区刷出。
- 成功写入时返回值始终在 0-255 范围内。

---

### 意图

将字符 `c` 写入标准输出流 `stdout`。等价于 `putc(c, stdout)` 或 `fputc(c, stdout)`。

Rust 侧实现：
- 直接委托给 `do_putc(c, stdout)`，`stdout` 为全局变量引用。

### 系统算法

```
putchar(c: c_int) -> c_int:
  return do_putc(c, stdout)
```

参见 `putc.h` 对应的 Rust spec 中 `do_putc` 的完整算法描述。

时间复杂度 O(1)（不含底层 I/O）。

---

## 依赖图

```
putchar (Public)
  └── do_putc (内部 inline 函数, from "putc.rs" 内部模块)
        ├── putc_unlocked (内部宏/内联函数)
        │     └── __overflow
        ├── locking_putc (内部 static 函数)
        │     ├── __lockfile / atomic CAS / __wake
        │     └── putc_unlocked
        └── __pthread_self

stdout (全局变量, 来自 stdio 模块)
```

---

## [RELY]

- `do_putc(c_int, *mut FILE)` — 内部字符写入函数（`putc.rs` 内部模块）。
- `stdout` — 标准输出 `*mut FILE` 指针（stdio 模块全局变量）。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn putchar(c: c_int) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为符合 ISO C 标准 `putchar` 语义。
