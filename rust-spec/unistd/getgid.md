# getgid — Rust 接口归约

## 原始 C 接口
```c
gid_t getgid(void);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn getgid() -> core::ffi::c_uint;
```

---

## 意图

获取调用进程的真实组 ID（Real GID）。该调用是 `SYS_getgid` 系统调用的零参数薄封装，总是成功。

## 前置条件

- 无

## 后置条件

- **Case 1 唯一情况 — 总是成功**
  - 返回调用进程的真实组 ID
  - 该值在进程生命周期内通常不变（除非通过 `setgid`/`setregid`/`setresgid` 修改）

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现直接委托 `__syscall(SYS_getgid)`。Rust 中直接调用底层 syscall：

```rust
#[inline]
unsafe fn sys_getgid() -> u32 {
    // x86_64: syscall!(SYS_getgid)
    // aarch64: svc #0 with x8 = SYS_getgid
}
```

---

## Rust 安全包装（模块内部）

```rust
// 安全包装，总是成功
pub(crate) fn get_real_gid() -> u32 {
    unsafe { sys_getgid() }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_getgid syscall — Linux 内核系统调用编号 (x86_64: 104, aarch64: 176)
Predefined Macros/Crates:
  syscall 模块 (rusl 内部 syscall 封装)

[GUARANTEE]
Exported Interface:
  extern "C" fn getgid() -> core::ffi::c_uint;
                                 // 本模块保证对外提供与 C ABI 兼容的 getgid 符号
Internal Interface:
  pub(crate) fn get_real_gid() -> u32;
                                 // 安全 Rust 包装
