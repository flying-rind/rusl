# setpgrp — Rust 接口归约

## 原始 C 接口
```c
pid_t setpgrp(void);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn setpgrp() -> core::ffi::c_int;
```

---

## 意图
将当前调用进程的进程组 ID 设置为自身 PID，等价于创建以自身为首进程的新进程组。该函数是 `setpgid(0, 0)` 的 BSD/XOPEN 便捷封装：将当前进程的 PGID 设为其自身 PID，从而将当前进程放入一个新的进程组（以自身为首进程）。

## 前置条件
- 调用进程未执行过 `execve`（否则子进程不允许修改自身进程组）
- 调用进程当前不在以其自身为首进程的进程组中
- 进程必须为其所在会话的成员

## 后置条件
- **Case 1 成功**
  - 当前进程的进程组 ID 被设置为当前进程的 PID
  - 当前进程成为新进程组的首进程
  - 返回新的进程组 ID（即当前进程的 PID）

- **Case 2 失败**
  - 返回 -1
  - `errno` 设置为对应错误码（如 `EPERM` — 调用进程是会话首进程且已在一个不同进程组中；或当前进程已执行过 `execve`）

## 与 setpgid 的关系

```
setpgrp() == setpgid(0, 0)
```

`setpgrp(void)` 是 BSD/XOPEN 的历史遗留接口，语义精确等价于 `setpgid(0, 0)`。musl 在 `<unistd.h>` 中同时提供两者，`setpgrp` 直接调用 `setpgid` 实现。

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现直接委托 `setpgid(0, 0)`。Rust 中：

```
setpgrp():
  return setpgid(0, 0)
```

即：
1. 调用 `setpgid(0, 0)`（同模块内 `setpgid` 的 POSIX 实现）
2. 参数 `pid=0` 表示当前进程，`pgid=0` 表示使用当前进程的 PID 作为新进程组 ID
3. 内部通过 `syscall(SYS_setpgid, 0, 0)` 执行内核系统调用

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  setpgid                             // 依赖1: 同模块内部函数（详见 setpgid.md）
  SYS_setpgid (syscall number)        // 依赖2: 底层 setpgid 系统调用 (x86_64: 109, aarch64: 154)
Predefined Macros/Crates:
  syscall! 宏或等效 no_std 封装      // 依赖3: 系统调用封装

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn setpgrp() -> core::ffi::c_int;
                                       // 本模块保证对外提供与 C ABI 兼容的 setpgrp 符号
