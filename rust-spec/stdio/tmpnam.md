# tmpnam 函数规约

## 复杂度分级: Level 1

> musl libc 标准库临时文件名生成函数（已过时）。生成一个唯一的临时文件路径名，不创建文件。Rust 实现中，外部接口保持 ABI 兼容，内部以安全 Rust 重构。

**安全警告**: 此函数在 POSIX.1-2008 中被标记为过时，存在 TOCTOU 竞态条件。C11 附录 K 建议使用 `tmpfile_s` 或 `mkstemp` 替代。但在 ABI 层面仍须提供兼容实现。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};

unsafe extern "C" fn tmpnam(buf: *mut c_char) -> *mut c_char;
```

[Visibility]: User — `<stdio.h>` 标准库函数（C89，已过时）。必须保持 ABI 兼容。参数 `buf` 可为 NULL（使用内部静态缓冲区），返回指向生成路径的指针或 NULL。

---

### 前置/后置条件

**[Pre-condition]:**
- 若 `buf` 非 NULL：指向至少 `L_tmpnam` 字节的可写内存空间。
- `/tmp` 目录存在且可访问。

**[Post-condition]:**
- **Case 1 成功生成唯一文件名（100 次尝试内）**
  - 返回指向包含 `/tmp/tmpnam_XXXXXX` 形式路径的缓冲区的指针（`buf` 或内部静态缓冲区）。
  - 该路径在设计上不与当前已有文件冲突。
  - 不创建文件，仅生成文件名。

- **Case 2 所有尝试失败**
  - 返回 `core::ptr::null_mut()`。
  - `errno` 设置为最后检测的系统调用错误码。

**[Error Behavior]:**
- 100 次尝试内无法生成唯一文件名时返回 NULL。内部静态缓冲区版本的并发调用存在数据竞争（此函数不要求线程安全，POSIX 已标记为过时）。

---

### 不变量

**[Invariant]:**
- 生成的路径名始终以 `/tmp/tmpnam_` 为前缀，后跟 6 个字母数字字符。
- 内部静态缓冲区 `static mut TMPNAM_INTERNAL: [u8; L_tmpnam]` 在不带参数调用时被重用，非线程安全。
- 最多 `MAXTRIES=100` 次尝试。
- 不创建任何文件。

---

### 意图

生成一个不与现有文件冲突的临时文件名。通过 `readlink` 系统调用来检测路径是否已存在（`readlink` 对不存在路径返回 `-ENOENT`）。

Rust 侧实现：
- 外部接口使用 `unsafe extern "C" fn tmpnam(buf: *mut c_char) -> *mut c_char`，保持 ABI 兼容。
- 内部使用固定字节数组 `b"/tmp/tmpnam_XXXXXX"` 作为模板。
- `__randname` 内部函数用 Rust 安全随机字符生成替代，接收 `&mut [u8]` 切片。
- `readlink` 存在检测使用内部 syscall 模块封装的安全接口。
- 当 `buf` 为 NULL 时，使用内部 `static mut TMPNAM_INTERNAL: [u8; L_tmpnam]` —— 注意：此全局可变静态变量的使用必须包裹在 `unsafe` 块中，符合 Rust 的安全约束。
- `strcpy` 替换为 Rust 安全字节复制（`copy_from_slice` + `write` 或在 unsafe 中使用 `ptr::copy_nonoverlapping`）。

### 系统算法

```
tmpnam(buf):
  internal: [u8; L_tmpnam] = [0; L_tmpnam]     // 内部静态缓冲区
  s: [u8; 22] = b"/tmp/tmpnam_XXXXXX"          // 固定前缀 + 随机占位符

  循环 MAXTRIES=100 次:
    1. __randname(&mut s[12..18])               // 将 s[12..17] 替换为随机字母数字
    2. r = readlink(s, &mut dummy, 1)           // 检测路径是否存在
       // readlink 在路径不存在时返回 -ENOENT
    3. 若 r == -ENOENT:                         // 路径不存在 = 可用
         将 s 复制到 dest (dest = buf 或 internal)
         返回 dest as *mut c_char

  返回 core::ptr::null_mut()  // 所有尝试失败
```

时间复杂度 O(MAXTRIES) 最坏情况，期望 O(1)。

---

## 依赖图

```
tmpnam (Public, extern "C")
  ├── core::ffi::{c_char, c_int}                           — Rust 内置 FFI 类型
  ├── [Internal] __randname(buf: &mut [u8])                 — 内部随机文件名生成
  ├── [Internal] syscall 模块 (sys_readlink / sys_readlinkat) — 内部安全 syscall
  ├── [Internal] L_tmpnam, ENOENT, AT_FDCWD                — 平台常量
  └── [Internal] __errno_location()                          — 设置 errno 的入口
```

---

## [RELY]

- `core::ffi::{c_char, c_int}` — Rust 核心库 FFI 类型。
- 内部 `__randname` — rusl 内部安全 Rust 实现。
- 内部 syscall 模块 — rusl 内部实现，封装 readlink 系统调用。
- `__errno_location()` — rusl 内部 errno 访问器。

## [GUARANTEE]

Exported Interface:
  `unsafe extern "C" fn tmpnam(buf: *mut c_char) -> *mut c_char;`

本模块保证对外提供上述 ABI 兼容的函数符号：
- 参数 `*mut c_char` 与 C `char *` 内存布局一致。
- 返回值 `*mut c_char` 与 C `char *` 内存布局一致。
- 使用 C 调用约定 (`extern "C"`)。
- 行为符合 C89 `tmpnam()` 语义：生成 `/tmp/tmpnam_XXXXXX` 形式的唯一路径名，最多 100 次重试。

**注意**: 此函数存在固有的 TOCTOU 竞态条件，调用者应优先使用 `mkstemp` 或 `tmpfile`。
