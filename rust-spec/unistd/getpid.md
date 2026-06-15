# getpid — Rust 接口归约

## 原始 C 接口
```c
pid_t getpid(void);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn getpid() -> core::ffi::c_int;
```

---

## 意图
获取当前调用进程的进程 ID（PID）。该调用是 `SYS_getpid` 系统调用的薄封装，直接返回内核原始返回值，因为此系统调用始终成功。

## 前置条件
- 无前置条件。此函数在任意进程上下文中均可调用。

## 后置条件
- **Case 1 始终成功**
  - 返回调用进程的进程 ID（正整数）
  - 返回值在进程的整个生命周期中保持不变
  - 永远不会失败，不会设置 `errno`

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现直接调用 `__syscall(SYS_getpid)` 并返回原始值（不经 `__syscall_ret` 转换）。Rust 中：

```
getpid():
  return syscall(SYS_getpid)
```

即：
1. 调用 `SYS_getpid` 系统调用
2. 直接返回内核原始值（进程 PID），因为此调用不会失败

---

## Rust 安全包装（模块内部）

```rust
/// 获取当前进程 ID。始终成功。
pub(crate) fn current_pid() -> i32 {
    unsafe { sys_getpid() }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_getpid (syscall number)         // 依赖1: 底层 getpid 系统调用 (x86_64: 39, aarch64: 172)
Predefined Macros/Crates:
  syscall! 宏或等效 no_std 封装      // 依赖2: 系统调用封装

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn getpid() -> core::ffi::c_int;
                                       // 本模块保证对外提供与 C ABI 兼容的 getpid 符号
Internal Interface:
  pub(crate) fn current_pid() -> i32;
                                       // 安全 Rust 包装，始终成功
