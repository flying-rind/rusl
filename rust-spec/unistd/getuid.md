# getuid — Rust 接口归约

## 原始 C 接口
```c
uid_t getuid(void);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn getuid() -> core::ffi::c_uint;
```

---

## 意图

获取调用进程的真实用户 ID（Real UID）。该调用是 `SYS_getuid` 系统调用的零参数薄封装，总是成功。Rust 侧通过 `extern "C"` 导出符号，内部直接调用 Linux `getuid` 系统调用。

## 前置条件

- 无

## 后置条件

- **Case 1 唯一情况 — 总是成功**
  - 返回调用进程的真实用户 ID
  - 该值在进程生命周期内通常不变（除非通过 `setuid`/`setreuid`/`setresuid` 修改）

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现直接委托 `__syscall(SYS_getuid)`。Rust 中：

```rust
// 直接调用 syscall SYS_getuid
#[inline]
unsafe fn sys_getuid() -> u32 {
    // x86_64: syscall!(SYS_getuid)
    // aarch64: svc #0 with x8 = SYS_getuid
}
```

由于 `getuid` 总是成功，无需错误处理，直接返回内核系统调用的结果即可。

---

## Rust 安全包装（模块内部）

```rust
// 安全包装，总是成功
pub(crate) fn get_real_uid() -> u32 {
    // 直接委托给 extern "C" 导出函数或内联 syscall
    unsafe { sys_getuid() }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_getuid syscall — Linux 内核系统调用编号 (x86_64: 102, aarch64: 174)
Predefined Macros/Crates:
  syscall 模块 (rusl 内部 syscall 封装)

[GUARANTEE]
Exported Interface:
  extern "C" fn getuid() -> core::ffi::c_uint;
                                 // 本模块保证对外提供与 C ABI 兼容的 getuid 符号
Internal Interface:
  pub(crate) fn get_real_uid() -> u32;
                                 // 安全 Rust 包装
