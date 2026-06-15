# geteuid — Rust 接口归约

## 原始 C 接口
```c
uid_t geteuid(void);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn geteuid() -> core::ffi::c_uint;
```

---

## 意图

获取调用进程的有效用户 ID（Effective UID）。该调用是 `SYS_geteuid` 系统调用的零参数薄封装，总是成功。

有效用户 ID 决定进程执行大多数操作（如文件访问、信号发送）时的权限。对于未设置 SUID 位的程序，有效用户 ID 与真实用户 ID 相同。对于设置了 SUID 位的程序，有效用户 ID 为文件所有者的用户 ID。

## 前置条件

- 无

## 后置条件

- **Case 1 唯一情况 — 总是成功**
  - 返回调用进程的有效用户 ID

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现直接委托 `__syscall(SYS_geteuid)`。Rust 中直接调用底层 syscall：

```rust
#[inline]
unsafe fn sys_geteuid() -> u32 {
    // x86_64: syscall!(SYS_geteuid)
    // aarch64: svc #0 with x8 = SYS_geteuid
}
```

---

## Rust 安全包装（模块内部）

```rust
// 安全包装，总是成功
pub(crate) fn get_effective_uid() -> u32 {
    unsafe { sys_geteuid() }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_geteuid syscall — Linux 内核系统调用编号 (x86_64: 107, aarch64: 175)
Predefined Macros/Crates:
  syscall 模块 (rusl 内部 syscall 封装)

[GUARANTEE]
Exported Interface:
  extern "C" fn geteuid() -> core::ffi::c_uint;
                                 // 本模块保证对外提供与 C ABI 兼容的 geteuid 符号
Internal Interface:
  pub(crate) fn get_effective_uid() -> u32;
                                 // 安全 Rust 包装
