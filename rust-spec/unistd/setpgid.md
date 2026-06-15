# setpgid — Rust 接口归约

## 原始 C 接口
```c
int setpgid(pid_t pid, pid_t pgid);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn setpgid(pid: core::ffi::c_int, pgid: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
将指定进程 `pid` 的进程组 ID 设置为 `pgid`。若 `pid` 为 0，则操作调用进程自身。若 `pgid` 为 0，则将 `pid` 的值用作进程组 ID。此操作用于将进程移动到新的或已存在的进程组中，或创建新的进程组。该调用是 `SYS_setpgid` 系统调用的薄封装。

## 前置条件
- `pid`: 有效的进程 ID，或 0（表示当前调用进程）
- `pgid`: 有效的进程组 ID，或 0（表示使用 `pid` 自身的值）
- 调用进程必须拥有适当的权限（与目标进程属于同一会话，或有 `CAP_SYS_ADMIN`）
- 目标进程与调用进程必须处于同一会话
- `pid` 不能是已执行过 `execve` 的子进程（防止竞争条件）

## 后置条件
- **Case 1 成功**
  - 进程 `pid` 的进程组 ID 被设置为 `pgid`（若 `pgid` 为 0 则为 `pid` 自身）
  - 返回 0

- **Case 2 参数无效**
  - `pid` 对应的进程不存在，或不在同一会话
  - 返回 -1，`errno` 设置为 `ESRCH`

- **Case 3 权限不足**
  - 调用进程没有足够权限修改目标进程的进程组
  - 返回 -1，`errno` 设置为 `EPERM`

- **Case 4 进程组 ID 无效**
  - `pgid` 指定的进程组不存在且不与 `pid` 相同
  - 返回 -1，`errno` 设置为 `EPERM`

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现调用 `syscall(SYS_setpgid, pid, pgid)` 进行系统调用并转换返回值。Rust 中：

```
setpgid(pid, pgid):
  ret = syscall(SYS_setpgid, pid, pgid)
  if ret < 0:
    errno = -ret
    return -1
  return 0
```

即：
1. 调用 `SYS_setpgid` 系统调用
2. 通过 `syscall_ret` 将内核返回值转换为 libc 约定（错误时设置 errno 返回 -1）

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_setpgid (syscall number)        // 依赖1: 底层 setpgid 系统调用 (x86_64: 109, aarch64: 154)
  syscall_ret 转换逻辑                // 依赖2: 内核返回值到 libc 约定的转换
Predefined Macros/Crates:
  syscall! 宏或等效 no_std 封装      // 依赖3: 系统调用封装

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn setpgid(pid: core::ffi::c_int, pgid: core::ffi::c_int) -> core::ffi::c_int;
                                       // 本模块保证对外提供与 C ABI 兼容的 setpgid 符号
