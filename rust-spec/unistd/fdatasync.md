# fdatasync — Rust 接口归约

## 原始 C 接口
```c
int fdatasync(int fd);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn fdatasync(fd: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
将文件描述符 `fd` 的已修改数据同步到磁盘。与 `fsync` 不同，`fdatasync` 不强制刷新元数据（除非元数据对后续读取必要，如文件大小变化），因此可能比 `fsync` 更高效。

## 前置条件
- `fd`: 有效的已打开文件描述符（`c_int`）

## 后置条件
- **Case 1 成功**: 所有缓冲数据已写入磁盘，返回 `0`
- **Case 2 错误**: 返回 `-1`，`errno` 设置为对应错误码

## 不变量
无。

## 算法
原 C 实现：`syscall_cp(SYS_fdatasync, fd)`。

```rust
#[inline]
unsafe fn sys_fdatasync(fd: core::ffi::c_int) -> core::ffi::c_int {
    // arch-specific syscall: SYS_fdatasync
    syscall!(SYS_fdatasync, fd) as core::ffi::c_int
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_fdatasync                               // 依赖1: Linux 系统调用编号
Predefined Macros/Crates:
  syscall! 宏                                 // 依赖2: 系统调用入口

[GUARANTEE]
Exported Interface:
  extern "C" fn fdatasync(fd: core::ffi::c_int) -> core::ffi::c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的 fdatasync 符号
Internal Interface:
  (无额外内部接口)
