# posix_close — Rust 接口归约

## 原始 C 接口
```c
int posix_close(int fd, int flags);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn posix_close(fd: core::ffi::c_int, flags: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
POSIX 标准文件描述符关闭函数。与 `close` 的区别在于支持 `flags` 参数。当前 musl 实现将 `flags` 忽略，直接委托给 `close`。

## 前置条件
- `fd`: 有效的已打开文件描述符（`c_int`）
- `flags`: 当前唯一有效值为 `POSIX_CLOSE_RESTART` (0)

## 后置条件
- 等同于 `close(fd)` 的后置条件
- **Case 1 成功**: 返回 `0`
- **Case 2 错误**: 返回 `-1`，`errno` 设置为对应错误码

## 不变量
无。

## 算法
原 C 实现：`return close(fd)`，直接委托给 `close`。

Rust 中：

```rust
#[inline]
unsafe fn sys_posix_close(fd: core::ffi::c_int, _flags: core::ffi::c_int) -> core::ffi::c_int {
    close(fd)  // 直接委托给 close
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  close(fd: core::ffi::c_int) -> core::ffi::c_int;
                                   // 依赖1: 标准 close 函数

[GUARANTEE]
Exported Interface:
  extern "C" fn posix_close(fd: core::ffi::c_int, flags: core::ffi::c_int) -> core::ffi::c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的 posix_close 符号
Internal Interface:
  (无额外内部接口)
