# tmpfile 函数规约

## 复杂度分级: Level 2

> musl libc 标准库临时文件创建函数。创建一个临时文件，当文件关闭或程序退出时自动删除。Rust 实现中，外部接口保持 ABI 兼容，内部文件名生成和文件操作使用 Rust 安全抽象。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为 rusl 内部类型，定义于 stdio_impl 模块
// 对外 ABI 中表现为 *mut FILE 不透明指针
unsafe extern "C" fn tmpfile() -> *mut FILE;
```

[Visibility]: User — `<stdio.h>` 标准库函数。必须保持 ABI 兼容。无参数，返回值 `*mut FILE`（不透明指针）表示成功创建的临时文件流，`core::ptr::null_mut()` 表示失败并设置 `errno`。

**注意**: `FILE` 类型为 rusl 内部结构体，外部代码仅通过不透明指针操作。`*mut FILE` 的内存布局与 C 的 `FILE *` 完全兼容。

---

### 前置/后置条件

**[Pre-condition]:**
- `/tmp` 目录存在且可写。
- 调用进程对 `/tmp` 拥有写权限和搜索权限。
- 系统有足够的 inode 和磁盘空间。

**[Post-condition]:**
- **Case 1 成功创建（100 次尝试内）**
  - 在 `/tmp` 下创建唯一命名的文件，以 `0600` 权限打开。
  - 文件的目录项立即被 `unlink`（仅通过 fd 访问）。
  - 返回读写双模式 `*mut FILE`（`w+` 模式）。
  - 调用者负责 `fclose`，关闭时文件数据自动释放。

- **Case 2 所有尝试失败**
  - 返回 `core::ptr::null_mut()`。
  - `errno` 设置为最后一个系统调用的错误码。

**[Error Behavior]:**
- 失败时返回 NULL 指针，`errno` 设置为对应的系统错误码（如 `EMFILE`、`ENOSPC`、`EACCES` 等）。

---

### 不变量

**[Invariant]:**
- 创建的文件权限始终为 `0600`（仅 owner 可读写）。
- 文件一旦 `unlink` 后即不可通过路径访问，仅通过返回的 `*mut FILE` 操作。
- 最多 `MAXTRIES=100` 次尝试。
- 使用 `O_CREAT | O_EXCL` 保证原子创建，避免 TOCTOU 竞态。

---

### 意图

在 `/tmp` 目录下创建一个临时文件，以 `"w+"`（读写）模式打开，并在创建后立即执行 `unlink` 操作。该文件在 `FILE` 关闭或程序退出时自动被系统回收。

Rust 侧实现：
- 外部接口使用 `unsafe extern "C" fn tmpfile() -> *mut FILE`，保持 ABI 兼容。
- 内部使用固定模板 `b"/tmp/tmpfile_XXXXXX"`，通过 `__randname` 内部函数替换尾部 6 个字符。
- `__randname` 内部函数使用 Rust 安全随机数生成（可复用 `rand` 相关 crate 或自行实现随机字符生成），无需保持与原 C `__randname` 的 ABI 一致。
- `sys_open` 系统调用通过内部 syscall 模块封装为安全接口。
- `unlink` 操作使用内部 syscall 模块（`SYS_unlink` 或 `SYS_unlinkat`）。
- `__fdopen` 内部使用 Rust 安全抽象从原始 fd 构造 FILE 对象。
- 若 `__fdopen` 失败，通过内部 syscall 模块安全关闭 fd。
- 重试循环使用 Rust 迭代器风格实现（如 `(0..MAXTRIES).find_map(...)`）。

### 系统算法

```
tmpfile():
  s = [u8; 22] = b"/tmp/tmpfile_XXXXXX"    // 固定前缀 + 6 位随机占位符

  循环 MAXTRIES=100 次:
    1. __randname(&mut s[13..19])             // 将 s[13..18] 替换为随机字母数字
    2. fd = sys_open(s, O_RDWR|O_CREAT|O_EXCL, 0600)  // 原子创建+打开
    3. 若 fd >= 0:
       a. unlink(s)                             // 立即删除目录项
       b. f = __fdopen(fd, "w+")               // 从 fd 创建 FILE 流
       c. 若 f 为空:
           关闭 fd                              // 手动清理
       d. 返回 f                                // 返回 FILE* (可能为空)
  循环结束
  返回 null_mut()  // 所有尝试失败
```

时间复杂度 O(MAXTRIES) 最坏情况，期望 O(1)。

---

## 依赖图

```
tmpfile (Public, extern "C")
  ├── core::ffi::c_int, core::ptr::null_mut        — Rust 内置类型
  ├── [Internal] __randname(buf: &mut [u8])         — 内部随机文件名生成 (可安全 Rust 实现)
  ├── [Internal] syscall 模块 (sys_open, sys_unlink/sys_unlinkat, sys_close) — 内部安全 syscall
  ├── [Internal] __fdopen(fd: c_int, mode: &str) -> *mut FILE  — 从 fd 构造 FILE
  ├── [Internal] FILE 类型                          — 定义于 stdio_impl 模块
  ├── [Internal] O_RDWR, O_CREAT, O_EXCL, AT_FDCWD — 平台常量
  └── [Internal] __errno_location()                  — 设置 errno 的入口
```

---

## [RELY]

- `core::ffi::c_int` — Rust 核心库 FFI 类型。
- 内部 syscall 模块 — rusl 内部实现，封装 Linux 文件操作系统调用。
- 内部 `__randname` — rusl 内部实现，可用 Rust 安全随机数生成替代原 C 实现。
- 内部 `__fdopen` — rusl 内部实现，从原始 fd 构造 FILE 对象。
- 内部 `FILE` 类型 — rusl stdio_impl 模块定义。
- `__errno_location()` — rusl 内部 errno 访问器。

## [GUARANTEE]

Exported Interface:
  `unsafe extern "C" fn tmpfile() -> *mut FILE;`

本模块保证对外提供上述 ABI 兼容的函数符号：
- 无参数，与 C `FILE *tmpfile(void)` 兼容。
- 返回值 `*mut FILE` 与 C `FILE *` 内存布局一致。
- 使用 C 调用约定 (`extern "C"`)。
- 行为符合 POSIX `tmpfile()` 语义：原子创建 + 立即 unlink，最多 100 次重试。
