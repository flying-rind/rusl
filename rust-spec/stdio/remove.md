# remove 函数规约

## 复杂度分级: Level 1

> musl libc 标准库文件删除函数。删除指定路径的文件或空目录。Rust 实现中，外部接口保持 ABI 兼容，内部 syscall 调用使用 Rust 安全抽象封装。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};

unsafe extern "C" fn remove(path: *const c_char) -> c_int;
```

[Visibility]: User — `<stdio.h>` 标准库函数。必须保持 ABI 兼容，外部 C 代码可直接链接调用。参数 `path` 为以 NUL 结尾的 C 字符串指针，返回值 `0` 表示成功，`-1` 表示失败并设置 `errno`。

---

### 前置/后置条件

**[Pre-condition]:**
- `path`: 非空指针，指向以 NUL 结尾的有效路径字符串。
- 调用进程对 `path` 所在目录具有写权限和搜索权限。
- 若 `path` 指向目录，该目录必须为空。

**[Post-condition]:**
- **Case 1 成功删除文件或空目录**
  - `path` 对应的文件/目录从文件系统中移除。
  - 返回 `0`。
  - 不设置 `errno`。

- **Case 2 删除失败**
  - 路径不存在 -> `errno = ENOENT`。
  - 权限不足 -> `errno = EACCES` 或 `EPERM`。
  - 目录非空 -> `errno = ENOTEMPTY`。
  - `path` 为正在使用的文件 -> `errno = EBUSY`。
  - 返回 `-1`。

**[Error Behavior]:**
- 失败时返回 `-1`，`errno` 设置为对应错误码。调用者应通过 `__errno_location()` 读取 `errno` 判断具体失败原因。

---

### 不变量

**[Invariant]:**
- 若 `path` 为目录，仅当目录为空时删除成功。
- 不可逆操作：文件一旦删除无法通过 libc 恢复。
- 不修改传入的路径字符串。

---

### 意图

从文件系统中删除 `path` 指向的文件或空目录。先尝试作为文件删除（`unlink`），若因 `EISDIR`（目标为目录）失败则尝试作为空目录删除（`rmdir`）。等价于对文件调用 `unlink()`，对目录调用 `rmdir()`。

Rust 侧实现：
- 外部接口使用 `unsafe extern "C" fn remove(path: *const c_char) -> c_int`，保持与 C ABI 完全兼容。
- 内部实现将 `path` 转换为 Rust `&CStr` 或字节切片，传入内部安全 syscall 抽象层。
- 内部 syscall 层将内核返回的负错误码（Linux 约定）转换为用户空间返回值：成功返回 `0`，失败返回 `-1` 并设置 `errno`。
- 编译时条件（`SYS_unlink`/`SYS_rmdir` 是否可用）通过 Rust `#[cfg]` 属性或 feature 标志处理，选择对应的 syscall 路径。

### 系统算法

```
remove(path):
  1. 尝试文件删除:
     若系统支持 SYS_unlink:
       调用内部 sys_unlink(path) 系统调用封装
     否则:
       调用内部 sys_unlinkat(AT_FDCWD, path, 0) 系统调用封装

  2. 若步骤1返回 -EISDIR (目标为目录):
     若系统支持 SYS_rmdir:
       调用内部 sys_rmdir(path) 系统调用封装
     否则:
       调用内部 sys_unlinkat(AT_FDCWD, path, AT_REMOVEDIR) 系统调用封装

  3. 将内核负错误码转换为: 成功返回 0，失败返回 -1 并设置 errno
```

时间复杂度 O(1)（不计内核 syscall 开销）。

---

## 依赖图

```
remove (Public, extern "C")
  ├── core::ffi::{c_char, c_int}                     — Rust 内置 FFI 类型
  ├── [Internal] syscall 模块 (sys_unlink / sys_unlinkat / sys_rmdir)  — 内部安全 syscall 封装
  ├── [Internal] __errno_location()                   — 设置 errno 的入口
  └── [Internal] AT_FDCWD, AT_REMOVEDIR, EISDIR      — 平台常量 (可通过 Rust 常量模块定义)
```

---

## [RELY]

- `core::ffi::{c_char, c_int}` — Rust 核心库 FFI 类型，无外部 crate 依赖。
- 内部 syscall 模块 — rusl 内部实现，封装 Linux `SYS_unlink` / `SYS_unlinkat` / `SYS_rmdir` 系统调用。
- `__errno_location()` — rusl 内部 errno 访问器，用于设置错误码。

## [GUARANTEE]

Exported Interface:
  `unsafe extern "C" fn remove(path: *const c_char) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号：
- 参数类型布局 (`*const c_char`) 与 C 的 `const char *` 完全一致。
- 返回值类型 (`c_int`) 与 C 的 `int` 完全一致。
- 使用 C 调用约定 (`extern "C"`)。
- 行为符合 POSIX `remove()` 语义：先 `unlink` 后 `rmdir` 的 fallback 逻辑与原 musl 实现一致。
