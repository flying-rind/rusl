# lseek — Rust 接口归约

## 原始 C 接口
```c
off_t __lseek(int fd, off_t offset, int whence);
// weak_alias(__lseek, lseek);
off_t lseek(int fd, off_t offset, int whence);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn __lseek(fd: core::ffi::c_int, offset: i64, whence: core::ffi::c_int) -> i64;
extern "C" fn lseek(fd: core::ffi::c_int, offset: i64, whence: core::ffi::c_int) -> i64;
// lseek 是 __lseek 的弱别名，两者导出同一符号
```

> **注意**: `off_t` 在 64 位平台上为 `i64`，在 32 位平台上也为 `i64`（musl 使用 64 位 off_t）。`lseek` 作为 `__lseek` 的弱别名，必须和 `__lseek` 一起导出。

---

## 意图
重新定位文件描述符 `fd` 的读写位置（文件偏移）。对于普通文件，新位置由 `offset` 和 `whence` 决定；对于管道、套接字等不可定位的文件描述符则返回错误。

## 前置条件
- `fd`: 有效的已打开文件描述符（`c_int`）
- `whence`: `SEEK_SET` (0)、`SEEK_CUR` (1)、`SEEK_END` (2)、`SEEK_DATA` (3) 或 `SEEK_HOLE` (4)
- `offset`: 文件偏移量（有符号 `i64`）

## 后置条件
- **Case 1 成功**: 返回新的文件偏移位置（从文件开头的字节偏移，非负值），类型为 `i64`
- **Case 2 错误**: 返回 `-1`（`-1i64`），`errno` 设置为 `EBADF`、`EINVAL`、`ESPIPE`、`EOVERFLOW` 等

## 不变量
无。

## 算法
原 C 实现：
```
__lseek(fd, offset, whence):
  #ifdef SYS__llseek:                 // 32 位平台
    off_t result
    ret = syscall(SYS__llseek, fd, offset>>32, offset, &result, whence)
    return ret ? -1 : result
  #else:                              // 64 位平台
    return syscall(SYS_lseek, fd, offset, whence)
  #endif
```

Rust 中可通过条件编译实现：

```rust
#[inline]
unsafe fn sys_lseek(fd: core::ffi::c_int, offset: i64, whence: core::ffi::c_int) -> i64 {
    #[cfg(target_pointer_width = "64")]
    {
        syscall!(SYS_lseek, fd, offset, whence) as i64
    }
    #[cfg(target_pointer_width = "32")]
    {
        let mut result: i64 = 0;
        let ret = syscall!(SYS__llseek, fd, (offset >> 32) as i32, offset as i32,
                           &mut result as *mut i64, whence);
        if ret != 0 { -1 } else { result }
    }
}
```

---

## Rust 安全包装（模块内部）

```rust
pub(crate) fn seek(fd: core::ffi::c_int, offset: i64, whence: core::ffi::c_int) -> Result<i64, Error> {
    let ret = unsafe { lseek(fd, offset, whence) };
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
  SYS_lseek  (64 位平台)                      // 依赖1: Linux lseek 系统调用
  SYS__llseek (32 位平台)                     // 依赖2: Linux 扩展 llseek 系统调用
Predefined Macros/Crates:
  target_pointer_width 编译时条件              // 依赖3: 32/64 位平台区分

[GUARANTEE]
Exported Interface:
  extern "C" fn __lseek(fd: core::ffi::c_int, offset: i64, whence: core::ffi::c_int) -> i64;
  extern "C" fn lseek(fd: core::ffi::c_int, offset: i64, whence: core::ffi::c_int) -> i64;
                                   // 本模块保证对外提供与 C ABI 兼容的 __lseek/lseek 符号
                                   // lseek 是 __lseek 的弱别名
Internal Interface:
  pub(crate) fn seek(fd: core::ffi::c_int, offset: i64, whence: core::ffi::c_int) -> Result<i64, Error>;
                                   // 安全 Rust 包装
