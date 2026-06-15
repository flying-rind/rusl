# dup3 — Rust 接口归约

## 原始 C 接口
```c
int __dup3(int old, int new, int flags);
// weak_alias(__dup3, dup3);
int dup3(int old, int new, int flags);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn __dup3(old: core::ffi::c_int, new: core::ffi::c_int, flags: core::ffi::c_int) -> core::ffi::c_int;
extern "C" fn dup3(old: core::ffi::c_int, new: core::ffi::c_int, flags: core::ffi::c_int) -> core::ffi::c_int;
// dup3 是 __dup3 的弱别名，两者导出同一符号
```

---

## 意图
等价于 `dup2(old, new)`，但额外支持 `flags` 参数控制新文件描述符的行为。当前支持的 flag 为 `O_CLOEXEC`（设置 close-on-exec 标志）。与 dup2 不同，`dup3(old, new, flags)` 要求 `old != new`。

## 前置条件
- `old`: 有效的已打开文件描述符（`c_int`）
- `new`: `[0, OPEN_MAX)` 范围内的目标文件描述符编号（`c_int`）
- `old != new`: 不允许 `old == new`（与 dup2 不同）
- `flags`: `O_CLOEXEC` 或其组合（Linux 5.3+ 无此限制）

## 后置条件
- **Case 1 成功**: `new` 成为 `old` 的副本，若 `flags & O_CLOEXEC` 则设置 close-on-exec 标志，返回 `new`
- **Case 2 `old == new`**: 返回 `-1`，`errno` 设置为 `EINVAL`
- **Case 3 `old` 无效或 `new` 超出范围**: 返回 `-1`，`errno` 设置为 `EBADF`
- **Case 4 `flags` 包含不支持的标志**: 返回 `-1`，`errno` 设置为 `EINVAL`

## 不变量
无。

## 算法
原 C 实现根据平台分为多条路径：
```
__dup3(old, new, flags):
  #ifdef SYS_dup2:    // 有 dup2 的平台
    if old == new: return -EINVAL
    if flags:
      r = __syscall(SYS_dup3, old, new, flags)   // 优先尝试 dup3
      if r != -ENOSYS: return r
      if flags & ~O_CLOEXEC: return -EINVAL       // 回退仅支持 O_CLOEXEC
    while ((r=__syscall(SYS_dup2, old, new)) == -EBUSY);  // dup2 + 手动 fcntl
    if r >= 0 && (flags & O_CLOEXEC):
      __syscall(SYS_fcntl, new, F_SETFD, FD_CLOEXEC)
  #else:               // 无 dup2 的平台
    while ((r=__syscall(SYS_dup3, old, new, flags)) == -EBUSY);
  return __syscall_ret(r)
```

Rust 中：

```rust
#[inline]
unsafe fn sys_dup3(old: core::ffi::c_int, new: core::ffi::c_int, flags: core::ffi::c_int) -> core::ffi::c_int {
    #[cfg(any(target_arch = "x86_64", target_arch = "x86", target_arch = "riscv64"))]
    {
        if old == new { return -1; /* errno = EINVAL */ }
        if flags != 0 {
            let r = syscall!(SYS_dup3, old, new, flags);
            if r != -(ENOSYS as isize) { return r as core::ffi::c_int; }
            if flags & !(O_CLOEXEC as core::ffi::c_int) != 0 { return -1; /* EINVAL */ }
        }
        loop {
            let r = syscall!(SYS_dup2, old, new);
            if r != -(EBUSY as isize) {
                if r >= 0 && (flags & (O_CLOEXEC as core::ffi::c_int)) != 0 {
                    syscall!(SYS_fcntl, new, F_SETFD, FD_CLOEXEC);
                }
                return r as core::ffi::c_int;
            }
        }
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "riscv64")))]
    {
        loop {
            let r = syscall!(SYS_dup3, old, new, flags);
            if r != -(EBUSY as isize) { return r as core::ffi::c_int; }
        }
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_dup3                                    // 依赖1: Linux dup3 系统调用
  SYS_dup2  (有 dup2 的平台)                  // 依赖2: Linux dup2 系统调用（回退）
  SYS_fcntl + F_SETFD + FD_CLOEXEC            // 依赖3: 手动设置 close-on-exec
Predefined Macros/Crates:
  O_CLOEXEC 标志常量                          // 依赖4: GNU/POSIX close-on-exec 标志

[GUARANTEE]
Exported Interface:
  extern "C" fn __dup3(old: core::ffi::c_int, new: core::ffi::c_int, flags: core::ffi::c_int) -> core::ffi::c_int;
  extern "C" fn dup3(old: core::ffi::c_int, new: core::ffi::c_int, flags: core::ffi::c_int) -> core::ffi::c_int;
                                   // dup3 是 __dup3 的弱别名
Internal Interface:
  (无额外内部接口)
