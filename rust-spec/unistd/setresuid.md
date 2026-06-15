# setresuid — Rust 接口归约

## 原始 C 接口
```c
#define _GNU_SOURCE
int setresuid(uid_t ruid, uid_t euid, uid_t suid);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数（GNU 扩展，需 _GNU_SOURCE）
extern "C" fn setresuid(ruid: core::ffi::c_uint, euid: core::ffi::c_uint, suid: core::ffi::c_uint) -> core::ffi::c_int;
```

---

## 意图

原子性地同时设置调用进程的真实用户 ID、有效用户 ID 和保存 set-user-ID。这是 Linux 特有的系统调用 `SYS_setresuid` 的封装，通过 `__setxid` 跨所有线程同步执行。

参数值为 -1（即 `(uid_t)-1`）表示对应的 ID 不被修改。与 `setuid` 和 `setreuid` 相比，`setresuid` 允许独立控制三个 ID。

## 前置条件

- `ruid`: 目标真实用户 ID（`uid_t` 类型），-1 表示不修改
- `euid`: 目标有效用户 ID（`uid_t` 类型），-1 表示不修改
- `suid`: 目标保存 set-user-ID（`uid_t` 类型），-1 表示不修改
- 需要定义 `_GNU_SOURCE` 特性测试宏以获取此接口声明

## 后置条件

- **Case 1 成功（特权进程或合法非特权设置）**
  - 如果 `ruid != (uid_t)-1`，进程的真实用户 ID 已更新
  - 如果 `euid != (uid_t)-1`，进程的有效用户 ID 已更新
  - 如果 `suid != (uid_t)-1`，进程的保存 set-user-ID 已更新
  - 返回 0

- **Case 2 权限不足**
  - 返回 -1
  - `errno` 设置为 `EPERM`

- **Case 3 所有线程同步失败**
  - 返回 -1
  - `errno` 设置为 `EAGAIN`

- **Case 4 线程间不一致（极端情况）**
  - 若某个线程设置成功而另一线程后续失败，进程被 `SIGKILL` 强制终止
  - 参见 [setxid.md](setxid.md)

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现调用 `__setxid(SYS_setresuid, ruid, euid, suid)`。Rust 中：

```rust
extern "C" fn setresuid(ruid: core::ffi::c_uint, euid: core::ffi::c_uint, suid: core::ffi::c_uint) -> core::ffi::c_int {
    unsafe { __setxid(SYS_setresuid, ruid as i32, euid as i32, suid as i32) }
}
```

`__setxid` 算法详见 [setxid.md](setxid.md) 规约。

---

## Rust 安全包装（模块内部）

```rust
// -1u32 表示不修改对应的 ID
pub(crate) fn set_res_uid(ruid: u32, euid: u32, suid: u32) -> Result<(), Errno> {
    let ret = unsafe { setresuid(ruid, euid, suid) };
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
  extern "C" fn setresuid(ruid: core::ffi::c_uint, euid: core::ffi::c_uint, suid: core::ffi::c_uint) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 setresuid 符号（GNU 扩展）
Internal Interface:
  pub(crate) fn set_res_uid(ruid: u32, euid: u32, suid: u32) -> Result<(), Errno>;
                                 // 安全 Rust 包装
