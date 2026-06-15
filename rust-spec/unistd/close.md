# close — Rust 接口归约

## 原始 C 接口
```c
int close(int fd);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn close(fd: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
关闭文件描述符 `fd`，释放与之关联的内核资源。在关闭前调用 AIO 关闭回调，并特殊处理 `EINTR` 错误（POSIX 要求 close 在 `EINTR` 时不重试，但 Linux 内核的实际行为是重试后成功，musl 将 `EINTR` 视为成功）。

## 前置条件
- `fd`: 有效的已打开文件描述符（`c_int`）

## 后置条件
- **Case 1 成功关闭**: `fd` 不再可用，资源被释放，返回 `0`
- **Case 2 `fd` 无效**: 返回 `-1`，`errno` 设置为 `EBADF`
- **Case 3 被 `EINTR` 中断**: 视为成功，返回 `0`（符合 Linux 内核语义）

## 不变量
无。

## 算法
原 C 实现：
```
close(fd):
  fd = __aio_close(fd)          // 1. AIO 关闭回调（默认无操作）
  r = __syscall_cp(SYS_close, fd) // 2. 执行内核 close 系统调用
  if r == -EINTR: r = 0          // 3. EINTR 视为成功
  return __syscall_ret(r)        // 4. 转换为 libc 约定
```

Rust 中：

```rust
#[inline]
unsafe fn sys_close(fd: core::ffi::c_int) -> core::ffi::c_int {
    // arch-specific syscall: SYS_close (x86_64: 3, aarch64: 57)
    let r = syscall!(SYS_close, fd);
    if r == -(EINTR as isize) { 0 } else { r as core::ffi::c_int }
}
```

`__aio_close` 为弱符号，若未链接 AIO 实现则为恒等函数 `|fd| fd`。Rust 中可将 AIO 回调作为可选特性，通过 feature gate 控制。

---

## Rust 安全包装（模块内部）

```rust
pub(crate) fn close_fd(fd: core::ffi::c_int) -> Result<(), Error> {
    let ret = unsafe { close(fd) };
    if ret < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_close                                   // 依赖1: Linux 系统调用编号 (x86_64: 3, aarch64: 57)
  __aio_close (弱符号, 可选)                   // 依赖2: AIO 关闭回调，默认恒等函数
Predefined Macros/Crates:
  syscall! 宏                                 // 依赖3: 系统调用入口

[GUARANTEE]
Exported Interface:
  extern "C" fn close(fd: core::ffi::c_int) -> core::ffi::c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的 close 符号
Internal Interface:
  pub(crate) fn close_fd(fd: core::ffi::c_int) -> Result<(), Error>;
                                   // 安全 Rust 包装
