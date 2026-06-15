# getsid — Rust 接口归约

## 原始 C 接口
```c
pid_t getsid(pid_t pid);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn getsid(pid: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
获取指定进程 `pid` 的会话 ID（Session ID）。会话 ID 是创建该会话的进程（会话首进程）的 PID。若 `pid` 为 0，则获取调用进程自身的会话 ID。该调用是 `SYS_getsid` 系统调用的薄封装，使用 `syscall` 宏进行返回值转换以正确设置 `errno`。

## 前置条件
- `pid`: 有效的进程 ID，或 0（表示当前进程）

## 后置条件
- **Case 1 成功**
  - 返回指定进程的会话 ID（正整数，等于会话首进程的 PID）
  - 若 `pid` 为 0，返回调用进程自身的会话 ID

- **Case 2 `pid` 对应的进程不存在**
  - 返回 -1
  - `errno` 设置为 `ESRCH`

- **Case 3 无权限访问指定进程**
  - 返回 -1
  - `errno` 设置为 `EPERM`

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现调用 `syscall(SYS_getsid, pid)` 进行系统调用并转换返回值。Rust 中：

```
getsid(pid):
  ret = syscall(SYS_getsid, pid)
  if ret < 0:
    errno = -ret
    return -1
  return ret
```

即：
1. 调用 `SYS_getsid` 系统调用
2. 通过 `syscall_ret` 将内核返回值转换为 libc 约定（错误时设置 errno 返回 -1）

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_getsid (syscall number)         // 依赖1: 底层 getsid 系统调用 (x86_64: 124, aarch64: 156)
  syscall_ret 转换逻辑                // 依赖2: 内核返回值到 libc 约定的转换
Predefined Macros/Crates:
  syscall! 宏或等效 no_std 封装      // 依赖3: 系统调用封装

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn getsid(pid: core::ffi::c_int) -> core::ffi::c_int;
                                       // 本模块保证对外提供与 C ABI 兼容的 getsid 符号
