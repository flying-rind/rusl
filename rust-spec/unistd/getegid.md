# getegid — Rust 接口归约

## 原始 C 接口
```c
gid_t getegid(void);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn getegid() -> core::ffi::c_uint;
```

---

## 意图

获取调用进程的有效组 ID（Effective GID）。该调用是 `SYS_getegid` 系统调用的零参数薄封装，总是成功。

有效组 ID 决定进程执行文件访问等操作时的组权限。对于未设置 SGID 位的程序，有效组 ID 与真实组 ID 相同。对于设置了 SGID 位的程序，有效组 ID 为文件所属组的组 ID。

## 前置条件

- 无

## 后置条件

- **Case 1 唯一情况 — 总是成功**
  - 返回调用进程的有效组 ID

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现直接委托 `__syscall(SYS_getegid)`。Rust 中直接调用底层 syscall：

```rust
#[inline]
unsafe fn sys_getegid() -> u32 {
    // x86_64: syscall!(SYS_getegid)
    // aarch64: svc #0 with x8 = SYS_getegid
}
```

---

## Rust 安全包装（模块内部）

```rust
// 安全包装，总是成功
pub(crate) fn get_effective_gid() -> u32 {
    unsafe { sys_getegid() }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_getegid syscall — Linux 内核系统调用编号 (x86_64: 108, aarch64: 177)
Predefined Macros/Crates:
  syscall 模块 (rusl 内部 syscall 封装)

[GUARANTEE]
Exported Interface:
  extern "C" fn getegid() -> core::ffi::c_uint;
                                 // 本模块保证对外提供与 C ABI 兼容的 getegid 符号
Internal Interface:
  pub(crate) fn get_effective_gid() -> u32;
                                 // 安全 Rust 包装
