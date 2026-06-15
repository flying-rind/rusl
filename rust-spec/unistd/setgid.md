# setgid — Rust 接口归约

## 原始 C 接口
```c
int setgid(gid_t gid);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn setgid(gid: core::ffi::c_uint) -> core::ffi::c_int;
```

---

## 意图

将调用进程的组 ID 设置为指定值。内部调用 `__setxid(SYS_setgid, gid, 0, 0)` 跨所有线程同步执行 `SYS_setgid` 系统调用。

根据 POSIX 规范：
- 非特权进程只能将有效组 ID 设置为真实组 ID 或保存的 set-group-ID（附加组列表中的组也可以）
- 特权进程（egid == 0）设置真实、有效和保存的 set-group-ID 为 `gid`

## 前置条件

- `gid`: 目标组 ID（`gid_t` 类型）

## 后置条件

- **Case 1 成功（特权进程或合法非特权设置）**
  - 进程的组 ID 已更新
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

原 C 实现调用 `__setxid(SYS_setgid, gid, 0, 0)`。Rust 中：

```rust
extern "C" fn setgid(gid: core::ffi::c_uint) -> core::ffi::c_int {
    unsafe { __setxid(SYS_setgid, gid as i32, 0, 0) }
}
```

`__setxid` 算法详见 [setxid.md](setxid.md) 规约。

---

## Rust 安全包装（模块内部）

```rust
pub(crate) fn set_group_id(gid: u32) -> Result<(), Errno> {
    let ret = unsafe { setgid(gid) };
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
  SYS_setgid syscall — Linux 内核系统调用编号 (x86_64: 106, aarch64: 144)
Predefined Macros/Crates:
  syscall 模块 (rusl 内部 syscall 封装)

[GUARANTEE]
Exported Interface:
  extern "C" fn setgid(gid: core::ffi::c_uint) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 setgid 符号
Internal Interface:
  pub(crate) fn set_group_id(gid: u32) -> Result<(), Errno>;
                                 // 安全 Rust 包装
