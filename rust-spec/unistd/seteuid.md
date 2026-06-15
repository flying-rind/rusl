# seteuid — Rust 接口归约

## 原始 C 接口
```c
int seteuid(uid_t euid);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn seteuid(euid: core::ffi::c_uint) -> core::ffi::c_int;
```

---

## 意图

将调用进程的有效用户 ID 设置为指定值。实现上通过 `__setxid` 调用 `SYS_setresuid(-1, euid, -1)` —— 将真实用户 ID 和保存的 set-user-ID 设为 -1 表示不修改它们。

根据 POSIX 规范：
- 非特权进程只能将有效用户 ID 设置为真实用户 ID 或保存的 set-user-ID
- 特权进程（euid == 0）可以将有效用户 ID 设置为任意值

## 前置条件

- `euid`: 目标有效用户 ID（`uid_t` 类型）

## 后置条件

- **Case 1 成功（特权进程或合法非特权设置）**
  - 进程的有效用户 ID 已更新
  - 返回 0

- **Case 2 权限不足**
  - 返回 -1
  - `errno` 设置为 `EPERM`

- **Case 3 所有线程同步失败**
  - 返回 -1
  - `errno` 设置为 `EAGAIN`

- **Case 4 线程间不一致（极端情况）**
  - 若某个线程设置成功而另一线程后续失败，进程被 `SIGKILL` 强制终止
  - 参见 [setxid.md](setxid.md) 中 `do_setxid` 的安全终止逻辑

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现调用 `__setxid(SYS_setresuid, -1, euid, -1)`。Rust 中：

```rust
extern "C" fn seteuid(euid: core::ffi::c_uint) -> core::ffi::c_int {
    unsafe { __setxid(SYS_setresuid, -1i32 as i32, euid as i32, -1i32 as i32) }
}
```

通过将 `ruid` 和 `suid` 参数设为 -1 来保留真实用户 ID 和保存的 set-user-ID 不变。`__setxid` 算法详见 [setxid.md](setxid.md) 规约。

---

## Rust 安全包装（模块内部）

```rust
pub(crate) fn set_effective_uid(euid: u32) -> Result<(), Errno> {
    let ret = unsafe { seteuid(euid) };
    if ret == 0 {
        Ok(())
    } else {
        Err(Errno::from_last())
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __setxid(nr, id, eid, sid) — 跨线程同步执行 UID/GID 设置系统调用（内部函数，见 setxid.md）
  SYS_setresuid syscall — Linux 内核系统调用编号 (x86_64: 117, aarch64: 147)
Predefined Macros/Crates:
  syscall 模块 (rusl 内部 syscall 封装)

[GUARANTEE]
Exported Interface:
  extern "C" fn seteuid(euid: core::ffi::c_uint) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 seteuid 符号
Internal Interface:
  pub(crate) fn set_effective_uid(euid: u32) -> Result<(), Errno>;
                                 // 安全 Rust 包装
