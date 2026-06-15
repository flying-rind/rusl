# setsid — Rust 接口归约

## 原始 C 接口
```c
pid_t setsid(void);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn setsid() -> core::ffi::c_int;
```

---

## 意图
创建一个新会话（session）并将调用进程设为该会话的首进程。同时，调用进程成为新进程组的首进程，并且与之前的控制终端（controlling terminal）断开连接。这一操作是实现守护进程（daemon）化的关键步骤之一。该调用是 `SYS_setsid` 系统调用的薄封装。

## 前置条件
- 调用进程不能已经是某个进程组的首进程（否则操作无意义，内核会拒绝）
- 进程必须具有创建新会话的权限（普通用户进程通常具备）

## 后置条件
- **Case 1 成功**
  - 调用进程成为一个新会话的会话首进程
  - 调用进程成为一个新进程组的进程组首进程
  - 调用进程不再拥有控制终端
  - 返回新创建的会话 ID（等于调用进程的 PID）

- **Case 2 调用进程已是进程组首进程**
  - 返回 -1
  - `errno` 设置为 `EPERM`

## 不变量
- 会话首进程同时也是其所在进程组的首进程
- 一个会话最多拥有一个控制终端
- 会话首进程退出时，会话中所有进程会收到 `SIGHUP` 信号

## 算法
原 C 实现调用 `syscall(SYS_setsid)` 进行系统调用并转换返回值。Rust 中：

```
setsid():
  ret = syscall(SYS_setsid)
  if ret < 0:
    errno = -ret
    return -1
  return ret
```

即：
1. 调用 `SYS_setsid` 系统调用
2. 通过 `syscall_ret` 将内核返回值转换为 libc 约定（错误时设置 errno 返回 -1）

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_setsid (syscall number)         // 依赖1: 底层 setsid 系统调用 (x86_64: 112, aarch64: 157)
  syscall_ret 转换逻辑                // 依赖2: 内核返回值到 libc 约定的转换
Predefined Macros/Crates:
  syscall! 宏或等效 no_std 封装      // 依赖3: 系统调用封装

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn setsid() -> core::ffi::c_int;
                                       // 本模块保证对外提供与 C ABI 兼容的 setsid 符号
