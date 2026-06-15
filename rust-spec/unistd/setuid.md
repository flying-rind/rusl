# setuid — Rust 接口归约

## 原始 C 接口
```c
int setuid(uid_t uid);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn setuid(uid: core::ffi::c_uint) -> core::ffi::c_int;
```

---

## 意图

将调用进程的用户 ID 设置为指定值。内部调用 `__setxid(SYS_setuid, uid, 0, 0)` 跨所有线程同步执行 `SYS_setuid` 系统调用，确保多线程环境下的安全性。

根据 POSIX 规范，`setuid` 的行为取决于调用者是否具有特权（`euid == 0`）：
- 非特权进程只能将有效/真实/保存的 set-user-ID 设置为当前真实用户 ID 或保存的 set-user-ID
- 特权进程（root）设置所有三个 ID（真实、有效、保存）为 `uid`

## 前置条件

- `uid`: 目标用户 ID（`uid_t` 类型，通常为 `unsigned int`）

## 后置条件

- **Case 1 成功（特权进程或合法非特权设置）**
  - 进程的用户 ID 已更新（具体哪些 ID 被修改取决于内核实现）
  - 返回 0

- **Case 2 权限不足**
  - 返回 -1
  - `errno` 设置为 `EPERM`

- **Case 3 所有线程同步失败**
  - 返回 -1
  - `errno` 设置为 `EAGAIN`
  - 原因：某个线程在 `__synccall` 执行期间因内核级错误失败

- **Case 4 线程间不一致（极端情况）**
  - 若某个线程设置成功而另一线程后续失败，进程将被 `SIGKILL` 强制终止
  - 这是一种安全措施：ID 设置必须原子完成，不一致状态是危险且不可恢复的

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现调用 `__setxid(SYS_setuid, uid, 0, 0)`。Rust 中：

```rust
extern "C" fn setuid(uid: core::ffi::c_uint) -> core::ffi::c_int {
    // 委托给内部 __setxid 函数
    unsafe { __setxid(SYS_setuid, uid as i32, 0, 0) }
}
```

`__setxid` 算法详见 [setxid.md](setxid.md) 规约。

---

## Rust 安全包装（模块内部）

```rust
// 安全包装，返回 Result
pub(crate) fn set_user_id(uid: u32) -> Result<(), Errno> {
    let ret = unsafe { setuid(uid) };
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
  SYS_setuid syscall — Linux 内核系统调用编号 (x86_64: 105, aarch64: 146)
Predefined Macros/Crates:
  syscall 模块 (rusl 内部 syscall 封装)
  __synccall 模块 (rusl 内部线程同步封装)

[GUARANTEE]
Exported Interface:
  extern "C" fn setuid(uid: core::ffi::c_uint) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 setuid 符号
Internal Interface:
  pub(crate) fn set_user_id(uid: u32) -> Result<(), Errno>;
                                 // 安全 Rust 包装
