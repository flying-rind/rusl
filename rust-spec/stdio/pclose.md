# pclose 函数规约

## 复杂度分级: Level 1

> musl libc 标准库管道关闭函数。关闭 `popen` 打开的流，等待子进程退出并返回其状态。Rust 实现中，外部接口保持 ABI 兼容，内部等待逻辑用 Rust 安全实现。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为 rusl 内部类型
unsafe extern "C" fn pclose(f: *mut FILE) -> c_int;
```

[Visibility]: User — `<stdio.h>` POSIX 标准函数。必须保持 ABI 兼容。`f` 必须是通过 `popen()` 成功打开的流。返回子进程的退出状态码，失败返回 `-1` 并设置 `errno`。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: 非空 `*mut FILE` 指针，必须是通过 `popen()` 成功打开的流。
- `f->pipe_pid` 存储了有效的子进程 PID。
- `f` 尚未被 `pclose` 关闭（重复关闭行为未定义）。

**[Post-condition]:**
- **Case 1 子进程正常退出**
  - `f` 被关闭，所有缓冲数据已刷新。
  - 子进程被回收（reaped），不再为僵尸进程。
  - 返回子进程的退出状态码（由 `waitpid` 报告，可被 `WIFEXITED`/`WEXITSTATUS` 等宏解析）。

- **Case 2 `waitpid` 失败**
  - `f` 仍被关闭。
  - `errno` 被设置（如 `ECHILD`、`EINTR` 后的真正错误）。
  - 返回 `-1`。

- **Case 3 `waitpid` 被 `EINTR` 中断**
  - 自动重试 `waitpid`（循环直到不为 `-EINTR`）。
  - 最终行为与 Case 1 或 Case 2 相同。

**[Error Behavior]:**
- `waitpid` 真正失败（非 `EINTR` 中断）时返回 `-1`，`errno` 设置为对应错误码。
- 对 `EINTR` 信号中断无限重试，确保子进程被回收。

---

### 不变量

**[Invariant]:**
- 调用后 `f` 不再有效（被 `fclose` 关闭）。
- 子进程被回收后不再为僵尸进程。
- 返回的 `status` 为 `waitpid` 原始状态值，不经 `__syscall_ret` 的 `-1 + errno` 转换。

---

### 意图

关闭 `popen` 打开的 FILE 流，等待关联的子进程退出，并返回子进程的终止状态。若子进程尚未退出，阻塞直到其退出。

Rust 侧实现：
- 外部接口使用 `unsafe extern "C" fn pclose(f: *mut FILE) -> c_int`，保持 ABI 兼容。
- 先从 FILE 结构体提取 `pipe_pid`（子进程 PID），使用 Rust 安全字段访问。
- 调用 `fclose(f)` 关闭流（内部先 flush 缓冲区再关闭 fd）。
- 使用内部 syscall 模块封装的 `wait4` 等待子进程退出。
- `EINTR` 重试循环使用 Rust `loop` 配合 `match` 实现，代码清晰且安全。
- 成功时直接返回 `status`，失败时返回 `-1`。

### 系统算法

```
pclose(f):
  1. pid = f.pipe_pid       // 提取子进程 PID

  2. fclose(f)              // 关闭 FILE: flush + close fd

  3. 循环等待子进程:
     loop {
       r = sys_wait4(pid, &mut status, 0, core::ptr::null_mut())
       若 r != -EINTR: break  // 非信号中断，退出循环
     }

  4. 若 r < 0:              // waitpid 真正失败
       返回 -1               // (errno 已设置)

  5. 返回 status            // 返回子进程退出状态
```

时间复杂度 O(1)（可能因等待子进程退出而阻塞）。

---

## 依赖图

```
pclose (Public, extern "C")
  ├── core::ffi::c_int                                    — Rust 内置 FFI 类型
  ├── [Internal] fclose(f: *mut FILE)                     — 关闭 FILE 流
  ├── [Internal] syscall 模块 (sys_wait4)                  — 内部安全 syscall
  ├── [Internal] EINTR                                     — 错误码常量
  └── [Internal] FILE 类型 (pipe_pid 字段)                  — stdio_impl 模块定义
```

---

## [RELY]

- `core::ffi::c_int` — Rust 核心库 FFI 类型。
- 内部 `fclose` — rusl 内部 FILE 关闭函数。
- 内部 syscall 模块 — rusl 内部实现，封装 Linux `wait4` 系统调用。
- `__errno_location()` — rusl 内部 errno 访问器。

## [GUARANTEE]

Exported Interface:
  `unsafe extern "C" fn pclose(f: *mut FILE) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号：
- 参数 `*mut FILE` 与 C `FILE *` 内存布局一致。
- 返回值 `c_int` 与 C `int` 完全一致。
- 使用 C 调用约定 (`extern "C"`)。
- 行为符合 POSIX `pclose()` 语义：关闭 popen 流，等待子进程退出，返回退出状态码。
- `EINTR` 自动重试逻辑与原 musl 实现一致。
