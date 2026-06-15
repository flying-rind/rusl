# dup — Rust 接口归约

## 原始 C 接口
```c
int dup(int fd);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn dup(fd: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
复制文件描述符 `fd`，返回一个新的文件描述符，该新描述符引用相同的打开文件描述、共享文件偏移和状态标志、具有最低的可用编号，且不共享 close-on-exec 标志（新 fd 的 FD_CLOEXEC 被清除）。

## 前置条件
- `fd`: 有效的已打开文件描述符（`c_int`）
- 进程的文件描述符数量未达到 `RLIMIT_NOFILE` 上限

## 后置条件
- **Case 1 成功**: 返回新的文件描述符（非负 `c_int`，不同于 `fd`），新 fd 与 `fd` 共享同一内核文件描述
- **Case 2 错误**: 返回 `-1`，`errno` 设置为 `EBADF`（fd 无效）或 `EMFILE`（达到进程 fd 上限）

## 不变量
无。

## 算法
原 C 实现直接委托 `syscall(SYS_dup, fd)`。使用 `syscall` 而非 `syscall_cp`，因为 dup 是快速操作，不需要取消点。

```rust
#[inline]
unsafe fn sys_dup(fd: core::ffi::c_int) -> core::ffi::c_int {
    // arch-specific syscall: SYS_dup
    syscall!(SYS_dup, fd) as core::ffi::c_int
}
```

---

## Rust 安全包装（模块内部）

```rust
pub(crate) fn dup_fd(fd: core::ffi::c_int) -> Result<core::ffi::c_int, Error> {
    let ret = unsafe { dup(fd) };
    if ret < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(ret)
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_dup                                     // 依赖1: Linux 系统调用编号
Predefined Macros/Crates:
  syscall! 宏                                 // 依赖2: 系统调用入口

[GUARANTEE]
Exported Interface:
  extern "C" fn dup(fd: core::ffi::c_int) -> core::ffi::c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的 dup 符号
Internal Interface:
  pub(crate) fn dup_fd(fd: core::ffi::c_int) -> Result<core::ffi::c_int, Error>;
                                   // 安全 Rust 包装
