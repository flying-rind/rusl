# setregid — Rust 接口归约

## 原始 C 接口
```c
int setregid(gid_t rgid, gid_t egid);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn setregid(rgid: core::ffi::c_uint, egid: core::ffi::c_uint) -> core::ffi::c_int;
```

---

## 意图

原子性地设置调用进程的真实组 ID 和有效组 ID。内部调用 `__setxid(SYS_setregid, rgid, egid, 0)` 跨所有线程同步执行 `SYS_setregid` 系统调用。

参数值为 -1（即 `(gid_t)-1`）表示对应的 ID 不被修改。非特权进程只能将真实组 ID 设置为当前真实组 ID 或有效组 ID，将有效组 ID 设置为当前真实组 ID、有效组 ID、保存的 set-group-ID 或某个附加组 ID。

## 前置条件

- `rgid`: 目标真实组 ID（`gid_t` 类型），-1 表示不修改
- `egid`: 目标有效组 ID（`gid_t` 类型），-1 表示不修改

## 后置条件

- **Case 1 成功（特权进程或合法非特权设置）**
  - 如果 `rgid != (gid_t)-1`，进程的真实组 ID 已更新
  - 如果 `egid != (gid_t)-1`，进程的有效组 ID 已更新
  - 对于非特权进程，有效组 ID 被设置为非 -1 值时，保存的 set-group-ID 也被设置为相同值
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

原 C 实现调用 `__setxid(SYS_setregid, rgid, egid, 0)`。Rust 中：

```rust
extern "C" fn setregid(rgid: core::ffi::c_uint, egid: core::ffi::c_uint) -> core::ffi::c_int {
    unsafe { __setxid(SYS_setregid, rgid as i32, egid as i32, 0) }
}
```

`__setxid` 算法详见 [setxid.md](setxid.md) 规约。

---

## Rust 安全包装（模块内部）

```rust
// -1u32 表示不修改对应的 ID
pub(crate) fn set_real_and_effective_gid(rgid: u32, egid: u32) -> Result<(), Errno> {
    let ret = unsafe { setregid(rgid, egid) };
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
  SYS_setregid syscall — Linux 内核系统调用编号 (x86_64: 114, aarch64: 143)
Predefined Macros/Crates:
  syscall 模块 (rusl 内部 syscall 封装)

[GUARANTEE]
Exported Interface:
  extern "C" fn setregid(rgid: core::ffi::c_uint, egid: core::ffi::c_uint) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 setregid 符号
Internal Interface:
  pub(crate) fn set_real_and_effective_gid(rgid: u32, egid: u32) -> Result<(), Errno>;
                                 // 安全 Rust 包装
