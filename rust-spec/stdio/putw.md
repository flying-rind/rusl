# putw 函数规约

## 复杂度分级: Level 1

> musl libc 整数二进制写入实现。将一个 int 整数的二进制表示写入 FILE 流。Rust 实现中，外部接口保持 ABI 兼容，内部委托给安全 I/O 函数。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为 rusl 内部类型
unsafe extern "C" fn putw(x: c_int, f: *mut FILE) -> c_int;
```

[Visibility]: User — SVID 兼容 / GNU 扩展函数，需定义 `_GNU_SOURCE` 方可使用。非 ISO C/POSIX 标准函数。必须保持 ABI 兼容。返回 `0` 表示成功，`EOF`（`-1`）表示失败。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: 非空 `*mut FILE` 指针，指向已打开的写模式流。
- `x`: 任意 `c_int` 值（写入其二进制表示）。

**[Post-condition]:**
- **Case 1 写入成功（全部 `sizeof(c_int)` 字节写入）**
  - `x` 的二进制表示已写入流 `f`。
  - 返回 `0`。

- **Case 2 写入失败（零或部分字节写入）**
  - 返回 `EOF`（`-1`）。
  - `f` 的错误标志可能被设置。

**[Error Behavior]:**
- 写入失败返回 `-1`（EOF），流错误标志可能被设置。

---

### 不变量

**[Invariant]:**
- 始终写入 `core::mem::size_of::<c_int>()` 字节（平台相关，通常为 4 字节）。
- 数据格式为原生字节序的二进制整数，不具备跨平台可移植性。
- `putw` 自身不锁定 FILE（锁由 `fwrite` 管理）。
- 这不是格式化 I/O，写入的是二进制字节而非文本。

---

### 意图

将 `c_int` 值 `x` 的底层二进制表示直接写入 FILE 流 `f`。

Rust 侧实现：
- 外部接口使用 `unsafe extern "C" fn putw(x: c_int, f: *mut FILE) -> c_int`，保持 ABI 兼容。
- 内部将 `x` 的字节表示写入流，委托给内部安全 `fwrite` 函数。
- 返回值为 `fwrite` 返回值映射：写入 1 个成功对象返回 `0`，写入 0 个对象返回 `-1`（EOF）。
- 实现极简：`fwrite(&x, size_of::<c_int>(), 1, f)` 后判断。

### 系统算法

```
putw(x, f):
  1. 调用内部 fwrite(&x, size_of::<c_int>(), 1, f)
  2. 若 fwrite 返回 1 (写入成功): 返回 0
  3. 若 fwrite 返回 0 (写入失败): 返回 -1 (EOF)
```

等价于 `fwrite(&x, size, 1, f) as c_int - 1`。

时间复杂度 O(sizeof(c_int))。

---

## 依赖图

```
putw (Public, extern "C")
  ├── core::ffi::c_int                  — Rust 内置 FFI 类型
  ├── core::mem::size_of                — Rust 编译时类型大小
  ├── [Internal] fwrite                 — 内部安全块写入函数
  └── [Internal] FILE 类型              — stdio_impl 模块定义
```

---

## [RELY]

- `core::ffi::c_int` — Rust 核心库 FFI 类型。
- 内部 `fwrite` — rusl 内部安全块写入函数。

## [GUARANTEE]

Exported Interface:
  `unsafe extern "C" fn putw(x: c_int, f: *mut FILE) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号：
- 参数 `c_int` 与 C `int` 完全一致，`*mut FILE` 与 C `FILE *` 内存布局一致。
- 返回值与 C `int` 完全一致。
- 使用 C 调用约定 (`extern "C"`)。
- 行为符合 SVID/GNU `putw()` 语义：将 int 的二进制表示写入流，成功返回 0，失败返回 EOF。

**注意**: 写入的是平台原生字节序的二进制数据，不具有跨平台可移植性。
