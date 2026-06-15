# fsync — Rust 接口归约

## 原始 C 接口
```c
int fsync(int fd);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn fsync(fd: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
将文件描述符 `fd` 的所有已修改数据和元数据（文件大小、访问时间等）同步到磁盘。确保系统崩溃后数据不会丢失。

## 前置条件
- `fd`: 有效的已打开文件描述符（通常需要写权限）（`c_int`）

## 后置条件
- **Case 1 成功**: 所有缓冲数据已写入磁盘，返回 `0`
- **Case 2 错误**: 返回 `-1`，`errno` 设置为 `EBADF`、`EIO`、`EROFS` 等

## 不变量
无。

## 算法
原 C 实现：`syscall_cp(SYS_fsync, fd)`。

```rust
#[inline]
unsafe fn sys_fsync(fd: core::ffi::c_int) -> core::ffi::c_int {
    // arch-specific syscall: SYS_fsync
    syscall!(SYS_fsync, fd) as core::ffi::c_int
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_fsync                                   // 依赖1: Linux 系统调用编号
Predefined Macros/Crates:
  syscall! 宏                                 // 依赖2: 系统调用入口

[GUARANTEE]
Exported Interface:
  extern "C" fn fsync(fd: core::ffi::c_int) -> core::ffi::c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的 fsync 符号
Internal Interface:
  (无额外内部接口)
