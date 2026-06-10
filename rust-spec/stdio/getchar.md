# getchar 函数规约

## 复杂度分级: Level 1

> musl libc 标准输入单字符读取实现。从 `stdin` 读取一个字符。

---

## 函数接口

```rust
use core::ffi::c_int;

// stdin 为全局变量，来自 stdio 模块
// use rusl_internal::stdin;

extern "C" fn getchar() -> c_int;
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用。必须保持 ABI 兼容：`extern "C"` 导出，无参数类型，返回值为 `c_int`。

---

### 前置/后置条件

**[Pre-condition]:**
- `stdin` 已正确初始化并处于可读状态。

**[Post-condition]:**
- **Case 1 成功读取字符**
  - 返回读取到的字符（0-255 的 `c_int` 值）。
  - `stdin` 流位置前进一个字符。

- **Case 2 到达文件末尾**
  - 返回 `EOF`（-1）。
  - `stdin` 设置 `F_EOF` 标志。

- **Case 3 读取错误**
  - 返回 `EOF`。
  - `stdin` 设置 `F_ERR` 标志。

**[Error Behavior]:**
- 到达 EOF 或读取错误时返回 `EOF`。

---

### 不变量

**[Invariant]:**
- 等价于 `getc(stdin)` 或 `fgetc(stdin)`。
- 成功读取时返回值始终在 0-255 范围内。

---

### 意图

从标准输入流 `stdin` 读取一个字符。等价于 `getc(stdin)` 或 `fgetc(stdin)`。

Rust 侧实现：
- 直接委托给 `do_getc(stdin)`，`stdin` 为全局变量引用。
- 函数签名无参数，只需返回 `c_int`。

### 系统算法

```
getchar() -> c_int:
  return do_getc(stdin)
```

参见 `getc.h` 对应的 Rust spec 中 `do_getc` 的完整算法描述。

时间复杂度 O(1)（不含底层 I/O）。

---

## 依赖图

```
getchar (Public)
  └── do_getc (内部 inline 函数, from "getc.rs" 内部模块)
        ├── getc_unlocked (内部宏/内联函数)
        │     └── __uflow
        ├── locking_getc (内部 static 函数)
        │     ├── __lockfile / atomic CAS / __wake
        │     └── getc_unlocked
        └── __pthread_self

stdin (全局变量, 来自 stdio 模块)
```

---

## [RELY]

- `do_getc(*mut FILE)` — 内部字符读取函数（`getc.rs` 内部模块）。
- `stdin` — 标准输入 `*mut FILE` 指针（stdio 模块全局变量）。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn getchar() -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为符合 ISO C 标准 `getchar` 语义。
