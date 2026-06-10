# fopen 函数规约

## 复杂度分级: Level 1

> musl libc 标准库文件打开函数。根据指定的文件名和模式字符串打开文件，返回关联的 `FILE*` 流。

---

## 函数接口

```rust
use core::ffi::{c_int, c_char};

// FILE 为 opaque 类型（定义同 fclose.rs spec）
#[repr(C)]
pub struct FILE { _private: [u8; 0] }

/// 根据文件名和模式打开文件，返回缓冲的 FILE 流。
/// - filename: 以 NULL 结尾的路径字符串
/// - mode: 以 NULL 结尾的模式字符串，首字符必须为 'r'/'w'/'a'
/// 返回新创建的 FILE 指针，失败返回 NULL。
unsafe extern "C" fn fopen(
    filename: *const c_char,
    mode: *const c_char,
) -> *mut FILE;
```

[Visibility]: `fopen` 声明于 `<stdio.h>`，是用户可直接调用的标准 C 库函数。在编译产物中以 `#[no_mangle]` 导出 `fopen` 符号，必须保持 ABI 兼容。

---

### 前置/后置条件

**[Pre-condition]:**
- `filename`: 一个以 NULL 结尾的有效路径字符串
- `mode`: 一个以 NULL 结尾的有效模式字符串，首字符必须为 `'r'`、`'w'` 或 `'a'`
- 可选 mode 后缀字符: `'+'`（读写）、`'x'`（排他创建）、`'e'`（close-on-exec）、`'b'`（二进制，无操作）

**[Post-condition]:**

- **Case 1: 成功** — 返回指向新分配的 `FILE` 对象的指针
  - 文件描述符已通过系统调用 `open(filename, flags, 0666)` 打开
  - 若 mode 中包含 `'e'` 且底层 `open()` 不支持原子 `O_CLOEXEC`，则通过 `fcntl(fd, F_SETFD, FD_CLOEXEC)` 额外设置 close-on-exec 标志
  - `FILE` 对象已通过 `__fdopen` 初始化，包含缓冲区、操作函数指针等
  - `FILE` 对象已通过 `__ofl_add` 注册到全局打开文件链表

- **Case 2: 失败** — 返回 `NULL`
  - 若 mode 首字符不合法，设置 `errno = EINVAL`
  - 若 `sys_open` 失败，保持底层系统的 `errno` 值
  - 若 `__fdopen` 分配/初始化失败，已关闭文件描述符

**[Error Behavior]:**

| 条件 | errno 值 |
|------|----------|
| mode 首字符非 `r`/`w`/`a` | `EINVAL` |
| 文件打开失败 | 由 `sys_open` 设置（如 `ENOENT`, `EACCES` 等） |
| `__fdopen` 分配失败 | 由 alloc 模块设置 |

---

### 不变量

**[Invariant]:**
- 不会通过共享源路径的符号链接泄露控制权给其他进程（`O_CLOEXEC` 保证）
- 返回的 `*mut FILE` 在不再需要时必须由调用者通过 `fclose` 释放
- 若 `__fdopen` 失败，文件描述符已被关闭，无 fd 泄露

---

### 意图

根据 `filename` 指定的路径和 `mode` 指定的访问模式，创建并打开一个带缓冲的标准 I/O 流。这是用户打开文件最常用的入口函数。

Rust 侧实现：
- 外部接口 `fopen` 保持 `unsafe extern "C"` 的 ABI 签名
- 模式字符串解析使用 Rust 的字节操作（`*mode` 首字节匹配）替代 `strchr`
- `__fmodeflags` 调用将模式字符串转换为 `open()` 标志位（可内联或作为内部模块函数）
- 系统调用 `open` 通过 Rust 侧的 syscall 封装模块调用
- 清理路径使用 RAII：若 `__fdopen` 失败，在 `drop` 守卫中关闭 fd

### 系统算法

```
fopen(filename, mode):
  1. 校验 mode 首字符: 必须为 'r'/'w'/'a', 否则 errno=EINVAL, return NULL
  2. 调用 __fmodeflags(mode) 将模式字符串转换为 open(2) 标志位
  3. 调用 sys_open(filename, flags, 0666) 打开文件
     若 fd < 0, return NULL
  4. 若 flags 含 O_CLOEXEC，调用 sys_fcntl(fd, F_SETFD, FD_CLOEXEC)
     （弥补不支持原子 O_CLOEXEC 的内核）
  5. 调用 __fdopen(fd, mode) 创建 FILE 对象
     若失败:
        sys_close(fd)  // 清理文件描述符
        return NULL
  6. return f
```

时间复杂度 O(1)（不含系统调用开销）。

---

## 依赖图

```
fopen
  ├─> __fmodeflags           (see __fmodeflags.rs spec — 将 mode 字符串转换为 open() 标志)
  ├─> sys_open               (see syscall.rs — 系统调用 open)
  ├─> sys_fcntl              (see syscall.rs — 系统调用 fcntl)
  ├─> __fdopen               (see __fdopen.rs spec — 用文件描述符分配并初始化 FILE 对象)
  └─> 模式首字符校验          (内联字节匹配, 替代 C 的 strchr)
```

---

## [RELY]

- `__fmodeflags`: mode 字符串到 `open()` 标志位的转换（定义于 `rusl-stdio` 的 `__fmodeflags` 模块）
- `sys_open` / `sys_fcntl` / `sys_close`: 系统调用封装（定义于 `rusl` 的 syscall 模块）
- `__fdopen`: 从 fd 创建 FILE 对象（定义于 `rusl-stdio` 的 `__fdopen` 模块）

## [GUARANTEE]

Exported Interface:
```
unsafe extern "C" fn fopen(
    filename: *const c_char,
    mode: *const c_char,
) -> *mut FILE;
```

本模块保证对外提供 ABI 兼容的 `fopen` 符号。行为符合 C 标准库 `fopen()` 语义：校验模式字符串、通过系统调用打开文件、创建并初始化 `FILE` 对象。失败时保证无资源泄露（关闭 fd、释放内存）。

内部辅助函数（`__fmodeflags`、`__fdopen`）为独立模块的公共接口，不在此模块中暴露。
