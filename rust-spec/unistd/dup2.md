# dup2 — Rust 接口归约

## 原始 C 接口
```c
int dup2(int old, int new);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn dup2(old: core::ffi::c_int, new: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
复制文件描述符 `old`，使用 `new` 作为新文件描述符编号。如果 `new` 已打开，先关闭它。两个描述符共享同一内核文件描述。使用原子性的 dup2/dup3 系统调用避免竞态条件。

## 前置条件
- `old`: 有效的已打开文件描述符（`c_int`）
- `new`: `[0, OPEN_MAX)` 范围内的目标文件描述符编号（`c_int`）
- `old == new`: 允许（dup2 规范要求成功返回 new）

## 后置条件
- **Case 1 成功**: `new` 成为 `old` 的副本，`new` 的 FD_CLOEXEC 标志被清除，返回 `new`
- **Case 2 `old` 无效**: 返回 `-1`，`errno` 设置为 `EBADF`
- **Case 3 `new` 超出范围**: 返回 `-1`，`errno` 设置为 `EBADF`

## 不变量
无。

## 算法
原 C 实现根据平台分为两条路径：
```
dup2(old, new):
  #ifdef SYS_dup2:                               // 有 dup2 的平台
    while (__syscall(SYS_dup2, old, new) == -EBUSY);
    return __syscall_ret(r)
  #else:                                          // 无 dup2 的平台（如 aarch64）
    if old == new:
      r = __syscall(SYS_fcntl, old, F_GETFD)       // 仅检查 old 有效性
      if r >= 0: return old
    else:
      while (__syscall(SYS_dup3, old, new, 0) == -EBUSY) // 使用 dup3(old, new, 0)
      ;
    return __syscall_ret(r)
```

Rust 中：

```rust
#[inline]
unsafe fn sys_dup2(old: core::ffi::c_int, new: core::ffi::c_int) -> core::ffi::c_int {
    #[cfg(any(target_arch = "x86_64", target_arch = "x86", target_arch = "riscv64"))]
    {
        // 使用 SYS_dup2 的平台
        loop {
            let r = syscall!(SYS_dup2, old, new);
            if r != -(EBUSY as isize) { return r as core::ffi::c_int; }
        }
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "riscv64")))]
    {
        if old == new {
            let r = syscall!(SYS_fcntl, old, F_GETFD);
            return if r >= 0 { new } else { r as core::ffi::c_int };
        }
        loop {
            let r = syscall!(SYS_dup3, old, new, 0);
            if r != -(EBUSY as isize) { return r as core::ffi::c_int; }
        }
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_dup2  (有 dup2 的平台)                  // 依赖1: Linux dup2 系统调用
  SYS_dup3  (无 dup2 的平台)                  // 依赖2: Linux dup3 系统调用 (flags=0)
  SYS_fcntl + F_GETFD                        // 依赖3: 检查文件描述符有效性
Predefined Macros/Crates:
  target_arch 编译时条件                      // 依赖4: 区分平台

[GUARANTEE]
Exported Interface:
  extern "C" fn dup2(old: core::ffi::c_int, new: core::ffi::c_int) -> core::ffi::c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的 dup2 符号
Internal Interface:
  (无额外内部接口)
