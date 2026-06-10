# popen 函数规约

## 复杂度分级: Level 3

> musl libc 标准库管道执行函数。启动子进程执行 shell 命令，返回 FILE 流以读写其标准输入/输出。使用 `posix_spawn` 实现精细的文件描述符控制。Rust 实现中，外部接口保持 ABI 兼容，内部进程管理和文件操作使用 Rust 安全抽象。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};

// FILE 为 rusl 内部类型
unsafe extern "C" fn popen(cmd: *const c_char, mode: *const c_char) -> *mut FILE;
```

[Visibility]: User — `<stdio.h>` POSIX 标准函数。必须保持 ABI 兼容。`cmd` 为以 NUL 结尾的 shell 命令字符串，`mode` 以 `'r'` 或 `'w'` 开头，可选后跟 `'e'`。返回 `*mut FILE` 或 NULL（失败）。

---

### 前置/后置条件

**[Pre-condition]:**
- `cmd`: 非空指针，指向以 NUL 结尾的有效 shell 命令字符串。
- `mode`: 非空指针，必须以 `'r'`（读子进程输出）或 `'w'`（写子进程输入）开头；可选后跟 `'e'`（close-on-exec）。
- `/bin/sh` 可执行文件存在。
- 系统有足够的进程资源和文件描述符。

**[Post-condition]:**
- **Case 1 成功**
  - 创建子进程执行 `/bin/sh -c cmd`。
  - `mode="r"` 时：返回的 `*mut FILE` 连接子进程 stdout；子进程 stdout 被 dup 到管道写端。
  - `mode="w"` 时：返回的 `*mut FILE` 连接子进程 stdin；子进程 stdin 被 dup 到管道读端。
  - 返回的 FILE 的 `pipe_pid` 字段记录子进程 pid，供 `pclose` 使用。
  - 若不包含 `'e'` 标志，返回描述符的 close-on-exec 被清除。

- **Case 2 失败**
  - 返回 `core::ptr::null_mut()`。
  - `errno` 反映失败原因（可能为 `EINVAL`、`ENOMEM` 或系统调用错误码）。

**[Error Behavior]:**
- `mode` 不以 `'r'` 或 `'w'` 开头 -> `errno = EINVAL`，返回 NULL。
- 管道创建失败 -> 返回 NULL，errno 为 `pipe2` 的错误码。
- `fdopen` 失败 -> 返回 NULL，清理已打开 fd。
- `posix_spawn` 失败 -> 返回 NULL，清理 FILE 和 fd。

---

### 不变量

**[Invariant]:**
- 子进程始终通过 `/bin/sh -c` 执行，而非直接执行 `cmd`。
- 所有 popen 打开的、有 `pipe_pid` 的管道 fd 都会在 spawn 新子进程时关闭，防止描述符泄漏。
- 管道使用 `pipe2(O_CLOEXEC)` 创建，确保 exec 时自动关闭不使用的管道端。
- 使用 `posix_spawn`（而非 `fork`）实现更精细的文件描述符控制。

---

### 意图

创建管道、fork 子进程，子进程执行 `/bin/sh -c <cmd>`，父进程获得与子进程的标准输入/输出相连的 FILE 流。

Rust 侧实现：
- 外部接口使用 `unsafe extern "C" fn popen(cmd: *const c_char, mode: *const c_char) -> *mut FILE`，保持 ABI 兼容。
- 模式解析内部使用 Rust 字符串/字节匹配。
- `pipe2` 系统调用通过内部 syscall 模块封装。
- `fdopen` 内部使用 Rust 安全抽象从原始 fd 构造 FILE 对象。
- 遍历全局打开文件链表时，使用内部锁机制（`__ofl_lock`/`__ofl_unlock`）的 Rust 安全包装（如 `Mutex<Vec<...>>` 或内部链表锁）。
- `posix_spawn` 及其文件操作接口通过内部 syscall/进程管理模块封装。
- `fcntl` 的 close-on-exec 清除操作通过内部 syscall 模块封装。
- 错误路径使用 Rust 的 RAII 资源管理（如实现 `Drop` 的守卫类型自动关闭 fd），避免手动清理遗漏。

### 系统算法

```
popen(cmd, mode):
  1. 模式解析:
     若 *mode == 'r': op = 0 (父进程读)
     若 *mode == 'w': op = 1 (父进程写)
     否则: errno = EINVAL; 返回 NULL

  2. 创建管道:
     若 pipe2(p, O_CLOEXEC) != 0: 返回 NULL
     // p[0]=读端, p[1]=写端

  3. 从 fd 创建 FILE:
     f = fdopen(p[op], mode)
     若 f == NULL: 关闭 p[0], p[1]; 返回 NULL

  4. 初始化 posix_spawn 文件操作:
     若失败: 清理 FILE; 返回 NULL

     遍历所有打开的 FILE:
       若 FILE 有 pipe_pid 设置:
         在 spawn 前注册 close 操作（防止泄漏给子进程）

     注册 dup2 操作:
       op=0(父读): dup2(写端, stdout)
       op=1(父写): dup2(读端, stdin)

  5. 启动子进程:
     posix_spawn(&pid, "/bin/sh", &fa, 0,
                 ["sh", "-c", cmd, NULL],
                 environ)
     若成功:
       f.pipe_pid = pid
       若 mode 不含 'e': 清除 close-on-exec
       关闭不使用的管道端
       返回 f

  6. 错误: 清理并返回 NULL
```

---

## 依赖图

```
popen (Public, extern "C")
  ├── core::ffi::{c_char, c_int}                            — Rust 内置 FFI 类型
  ├── [Internal] syscall 模块 (pipe2, fcntl, close)          — 内部安全 syscall
  ├── [Internal] fdopen(fd, mode) -> *mut FILE               — 从 fd 构造 FILE
  ├── [Internal] fclose(f: *mut FILE)                        — 关闭 FILE 流
  ├── [Internal] __ofl_lock / __ofl_unlock                    — 全局打开文件链表锁
  ├── [Internal] posix_spawn 模块                            — 子进程创建
  ├── [Internal] strchr                                       — 字符查找，可用 Rust 替代
  ├── [Internal] __environ                                   — 环境变量数组
  └── [Internal] FILE 类型 (pipe_pid, fd, next 字段)          — stdio_impl 模块定义
```

---

## [RELY]

- `core::ffi::{c_char, c_int}` — Rust 核心库 FFI 类型。
- 内部 syscall 模块 — rusl 内部实现，封装 Linux 管道、文件、进程相关系统调用。
- 内部 `fdopen` / `fclose` — rusl 内部 FILE 管理。
- 内部 `__ofl_lock` / `__ofl_unlock` — rusl 内部打开文件列表锁（可用 Rust `Mutex` 安全替代内部实现）。
- 内部 posix_spawn 模块 — rusl 内部进程创建。
- 内部 `__environ` — rusl 内部环境变量全局数组。
- `__errno_location()` — rusl 内部 errno 访问器。

## [GUARANTEE]

Exported Interface:
  `unsafe extern "C" fn popen(cmd: *const c_char, mode: *const c_char) -> *mut FILE;`

本模块保证对外提供上述 ABI 兼容的函数符号：
- 参数类型布局与 C `const char *cmd, const char *mode` 完全一致。
- 返回值 `*mut FILE` 与 C `FILE *` 内存布局一致。
- 使用 C 调用约定 (`extern "C"`)。
- 行为符合 POSIX `popen()` 语义：通过 `/bin/sh -c` 执行命令，返回读写双工管道 FILE 流。
- 子进程使用 `posix_spawn` 创建，保证无文件描述符泄漏。
