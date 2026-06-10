# rename 函数规约

## 复杂度分级: Level 1

> musl libc 标准库文件重命名函数。将文件或目录从旧路径重命名为新路径。Rust 实现中，外部接口保持 ABI 兼容，内部 syscall 调用使用 Rust 安全抽象封装。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};

unsafe extern "C" fn rename(old: *const c_char, new: *const c_char) -> c_int;
```

[Visibility]: User — `<stdio.h>` 标准库函数。必须保持 ABI 兼容，外部 C 代码可直接链接调用。两个参数均为以 NUL 结尾的 C 字符串指针，返回值 `0` 表示成功，`-1` 表示失败并设置 `errno`。

---

### 前置/后置条件

**[Pre-condition]:**
- `old`: 非空指针，指向以 NUL 结尾的现有文件路径。
- `new`: 非空指针，指向以 NUL 结尾的目标路径；父目录必须存在。
- 调用进程对 `old` 和 `new` 的父目录具有写权限和搜索权限。

**[Post-condition]:**
- **Case 1 成功重命名**
  - `old` 路径不再指向该对象。
  - 该对象现在由 `new` 路径引用。
  - 返回 `0`。
  - 不设置 `errno`。

- **Case 2 重命名失败**
  - `old` 不存在 -> `errno = ENOENT`。
  - 权限不足 -> `errno = EACCES` 或 `EPERM`。
  - 跨文件系统重命名目录 -> `errno = EXDEV`。
  - `new` 为已存在目录但 `old` 为空目录 -> `errno = EISDIR` 或 `ENOTEMPTY`。
  - 返回 `-1`。

**[Error Behavior]:**
- 失败时返回 `-1`，`errno` 设置为对应错误码。

---

### 不变量

**[Invariant]:**
- 同一文件系统内的重命名保证原子性。
- 若 `new` 已存在，操作完成后原 `new` 文件不再存在（被原子替换）。
- 不修改传入的路径字符串。

---

### 意图

将文件系统对象从 `old` 路径重命名为 `new` 路径。若 `new` 已存在且为文件，则被原子替换。若 `old` 和 `new` 位于同一文件系统，操作为原子的；在不同文件系统之间通常失败（`EXDEV`）。

Rust 侧实现：
- 外部接口使用 `unsafe extern "C" fn rename(old: *const c_char, new: *const c_char) -> c_int`，保持 ABI 兼容。
- 内部将 C 字符串指针转换为 Rust 字节切片，传入内部 syscall 抽象层。
- 编译时根据系统支持情况选择 `SYS_rename`、`SYS_renameat` 或 `SYS_renameat2`，通过 `#[cfg]` 属性控制。
- 系统调用宏 `syscall(...)` 内部自动完成 `__syscall_ret` 错误码转换，Rust 侧对应为内部函数统一处理返回值和 errno 设置。

### 系统算法

```
rename(old, new):
  1. 优先尝试: syscall(SYS_rename, old, new)
  2. 若不支持 SYS_rename 但支持 SYS_renameat:
       syscall(SYS_renameat, AT_FDCWD, old, AT_FDCWD, new)
  3. 以上均不支持时 fallback:
       syscall(SYS_renameat2, AT_FDCWD, old, AT_FDCWD, new, 0)
  4. 内部统一处理返回值和 errno 设置
```

时间复杂度 O(1)（不计内核 syscall 开销）。

---

## 依赖图

```
rename (Public, extern "C")
  ├── core::ffi::{c_char, c_int}                                          — Rust 内置 FFI 类型
  ├── [Internal] syscall 模块 (sys_rename / sys_renameat / sys_renameat2) — 内部安全 syscall 封装
  ├── [Internal] __errno_location()                                        — 设置 errno 的入口
  └── [Internal] AT_FDCWD                                                  — 平台常量
```

---

## [RELY]

- `core::ffi::{c_char, c_int}` — Rust 核心库 FFI 类型。
- 内部 syscall 模块 — rusl 内部实现，封装 Linux `SYS_rename` / `SYS_renameat` / `SYS_renameat2` 系统调用。
- `__errno_location()` — rusl 内部 errno 访问器。

## [GUARANTEE]

Exported Interface:
  `unsafe extern "C" fn rename(old: *const c_char, new: *const c_char) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号：
- 参数类型布局与 C `const char *old, const char *new` 完全一致。
- 返回值为 `c_int`，与 C `int` 完全一致。
- 使用 C 调用约定 (`extern "C"`)。
- 行为符合 POSIX `rename()` 语义：同一文件系统内原子重命名，跨文件系统失败返回 `EXDEV`。
