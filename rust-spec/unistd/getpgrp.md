# getpgrp — Rust 接口归约

## 原始 C 接口
```c
pid_t getpgrp(void);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn getpgrp() -> core::ffi::c_int;
```

---

## 意图
获取当前调用进程的进程组 ID（PGID）。该函数是 `getpgid(0)` 的简化形式，等价于以参数 0 调用 `SYS_getpgid` 系统调用。直接返回内核原始值，因为查询自身进程组 ID 始终成功。

## 前置条件
- 无前置条件。此函数在任意进程上下文中均可调用。

## 后置条件
- **Case 1 始终成功**
  - 返回调用进程的进程组 ID（正整数）
  - 该值即创建进程组时的首进程 PID，或通过 `setpgid` 设置的进程组 ID
  - 永远不会失败，不会设置 `errno`

## 与 getpgid 的关系

```
getpgrp() == getpgid(0)
```

`getpgrp(void)` 是 POSIX 定义的无参版本，比 `getpgid(pid_t pid)` 更为简便。musl 实现中 `getpgrp` 直接使用 `__syscall` 而非 `syscall`，因为 `pid=0` 时系统调用不会失败，无需 `syscall_ret` 的 errno 转换。

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现以参数 `pid=0` 调用 `__syscall(SYS_getpgid, 0)` 并返回原始值。Rust 中：

```
getpgrp():
  return syscall(SYS_getpgid, 0)
```

即：
1. 以参数 `pid=0` 调用 `SYS_getpgid` 系统调用
2. PID 为 0 时，内核返回调用进程自身的进程组 ID
3. 直接返回内核原始值，因为此调用不会失败

---

## Rust 安全包装（模块内部）

```rust
/// 获取当前进程的进程组 ID。始终成功。
pub(crate) fn process_group() -> i32 {
    unsafe { sys_getpgid(0) }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_getpgid (syscall number)        // 依赖1: 底层 getpgid 系统调用 (x86_64: 121, aarch64: 155)
Predefined Macros/Crates:
  syscall! 宏或等效 no_std 封装      // 依赖2: 系统调用封装

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn getpgrp() -> core::ffi::c_int;
                                       // 本模块保证对外提供与 C ABI 兼容的 getpgrp 符号
Internal Interface:
  pub(crate) fn process_group() -> i32;
                                       // 安全 Rust 包装，始终成功
