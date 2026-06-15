# ftruncate — Rust 接口归约

## 原始 C 接口
```c
int ftruncate(int fd, off_t length);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn ftruncate(fd: core::ffi::c_int, length: i64) -> core::ffi::c_int;
```

> **注意**: `off_t` 在 musl 中始终为 64 位（`i64`）。`__SYSCALL_LL_O` 在 32 位平台上将 64 位 length 展开为两个 `long` 参数。

---

## 意图
将文件描述符 `fd` 引用的普通文件截断为精确 `length` 字节。若文件之前大于 `length`，超出部分被丢弃；若文件之前小于 `length`，扩展部分填充为零（形成稀疏文件）。

## 前置条件
- `fd`: 有效且以写模式打开的文件描述符（`c_int`）
- `length`: 非负的文件大小（`i64`）

## 后置条件
- **Case 1 成功**: 文件大小变为 `length`，返回 `0`
- **Case 2 错误**: 返回 `-1`，`errno` 设置为 `EBADF`、`EINVAL`、`EFBIG` 等

## 不变量
无。

## 算法
原 C 实现：`syscall(SYS_ftruncate, fd, __SYSCALL_LL_O(length))`。

Rust 中：

```rust
#[inline]
unsafe fn sys_ftruncate(fd: core::ffi::c_int, length: i64) -> core::ffi::c_int {
    #[cfg(target_pointer_width = "64")]
    {
        syscall!(SYS_ftruncate, fd, length) as core::ffi::c_int
    }
    #[cfg(target_pointer_width = "32")]
    {
        syscall!(SYS_ftruncate, fd, (length >> 32) as i32, length as i32) as core::ffi::c_int
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_ftruncate                               // 依赖1: Linux 系统调用编号
Predefined Macros/Crates:
  target_pointer_width 编译时条件              // 依赖2: 32/64 位长度适配
  syscall! 宏                                 // 依赖3: 系统调用入口

[GUARANTEE]
Exported Interface:
  extern "C" fn ftruncate(fd: core::ffi::c_int, length: i64) -> core::ffi::c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的 ftruncate 符号
Internal Interface:
  (无额外内部接口)
