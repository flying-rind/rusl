# getw 函数规约

## 复杂度分级: Level 1

> musl libc 整数二进制读取实现。从 FILE 流中读取一个 int 整数的二进制表示。Rust 实现中，外部接口保持 ABI 兼容，内部委托给安全 I/O 函数。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为 rusl 内部类型
unsafe extern "C" fn getw(f: *mut FILE) -> c_int;
```

[Visibility]: User — SVID 兼容 / GNU 扩展函数，需定义 `_GNU_SOURCE` 方可使用。非 ISO C/POSIX 标准函数。必须保持 ABI 兼容。返回读取的 int 值，失败返回 `EOF`（`-1`）。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: 非空 `*mut FILE` 指针，指向已打开的读模式流。

**[Post-condition]:**
- **Case 1 读取成功（完整读取 `sizeof(c_int)` 字节）**
  - 返回读取的 `c_int` 值。

- **Case 2 读取失败（0 字节或部分字节，或立即遇到 EOF）**
  - 返回 `EOF`（`-1`）。
  - `f` 的 EOF 或错误标志可能被设置。

**[Error Behavior]:**
- 读取失败返回 `-1`（EOF）。调用者无法仅通过返回值区分是真 EOF 还是读到了值 `-1`——需使用 `feof(f)` 或 `ferror(f)` 进行区分。

---

### 不变量

**[Invariant]:**
- 始终尝试读取恰好 `core::mem::size_of::<c_int>()` 字节。
- 读取的字节按平台原生字节序解释为 `c_int`。
- `getw` 自身不锁定 FILE（锁由 `fread` 管理）。
- 这是二进制读取，非格式化 I/O。

---

### 意图

从 FILE 流 `f` 中读取 `sizeof(c_int)` 字节的二进制数据，将其解释为一个 `c_int` 值并返回。

Rust 侧实现：
- 外部接口使用 `unsafe extern "C" fn getw(f: *mut FILE) -> c_int`，保持 ABI 兼容。
- 内部声明局部 `c_int` 变量，委托给内部安全 `fread` 函数读取。
- `fread` 返回 1（成功读取 1 个对象）时返回读取的值 `x`；返回 0 时返回 `EOF`（`-1`）。
- Rust 代码使用 `if fread(...) == 1 { x } else { -1 }` 模式。

### 系统算法

```
getw(f):
  1. let mut x: c_int = 0
  2. 若 fread(&mut x, size_of::<c_int>(), 1, f) == 1:
       返回 x
  3. 否则:
       返回 -1 (EOF)
```

时间复杂度 O(sizeof(c_int))。

---

## 依赖图

```
getw (Public, extern "C")
  ├── core::ffi::c_int                  — Rust 内置 FFI 类型
  ├── core::mem::size_of                — Rust 编译时类型大小
  ├── [Internal] fread                  — 内部安全块读取函数
  └── [Internal] EOF                    — 错误返回值常量
```

---

## [RELY]

- `core::ffi::c_int` — Rust 核心库 FFI 类型。
- 内部 `fread` — rusl 内部安全块读取函数。

## [GUARANTEE]

Exported Interface:
  `unsafe extern "C" fn getw(f: *mut FILE) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号：
- 参数 `*mut FILE` 与 C `FILE *` 内存布局一致。
- 返回值 `c_int` 与 C `int` 完全一致。
- 使用 C 调用约定 (`extern "C"`)。
- 行为符合 SVID/GNU `getw()` 语义：从流读取 sizeof(int) 字节的二进制数据。

**注意**: `EOF` 定义为 `-1`，而 `-1` 也是一个合法的 `c_int` 值。调用者需使用 `feof(f)` 或 `ferror(f)` 区分真正 EOF 和读取到值 `-1` 的情况。
