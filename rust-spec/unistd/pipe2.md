# pipe2 — Rust 接口归约

## 原始 C 接口
```c
int pipe2(int fd[2], int flag);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn pipe2(fd: *mut core::ffi::c_int, flag: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
创建一对管道文件描述符，并原子性地设置指定标志。比先 `pipe()` 再 `fcntl()` 更安全，避免了竞态条件。支持的 flag 包括 `O_CLOEXEC`（close-on-exec）和 `O_NONBLOCK`（非阻塞 I/O）。

## 前置条件
- `fd`: 指向至少 2 个 `c_int` 空间的有效非空指针
- `flag`: `0` 或 `O_CLOEXEC | O_NONBLOCK` 的组合

## 后置条件
- **Case 1 成功 (flag == 0)**: 行为等同于 `pipe(fd)`，返回 `0`
- **Case 2 成功 (flag != 0)**: `fd[0]` 为读端，`fd[1]` 为写端，根据 flag 设置对应标志，返回 `0`
- **Case 3 flag 包含不支持的位（且 SYS_pipe2 不可用）**: 返回 `-1`，`errno` 设置为 `EINVAL`
- **Case 4 其他错误**: 返回 `-1`，`errno` 设置为 `EMFILE`、`ENFILE` 等

## 不变量
无。

## 算法
原 C 实现：
```
pipe2(fd, flag):
  if !flag: return pipe(fd)                              // flag=0 直接委托 pipe

  ret = __syscall(SYS_pipe2, fd, flag)                    // 尝试原子的 pipe2
  if ret != -ENOSYS: return __syscall_ret(ret)

  // SYS_pipe2 不可用时的回退
  if flag & ~(O_CLOEXEC|O_NONBLOCK): return -EINVAL
  ret = pipe(fd)
  if ret: return ret

  if flag & O_CLOEXEC:
    __syscall(SYS_fcntl, fd[0], F_SETFD, FD_CLOEXEC)
    __syscall(SYS_fcntl, fd[1], F_SETFD, FD_CLOEXEC)
  if flag & O_NONBLOCK:
    __syscall(SYS_fcntl, fd[0], F_SETFL, O_NONBLOCK)
    __syscall(SYS_fcntl, fd[1], F_SETFL, O_NONBLOCK)
  return 0
```

Rust 中：

```rust
#[inline]
unsafe fn sys_pipe2(fd: *mut core::ffi::c_int, flag: core::ffi::c_int) -> core::ffi::c_int {
    if flag == 0 {
        return pipe(fd);  // 委托给 pipe
    }

    let ret = syscall!(SYS_pipe2, fd, flag);
    if ret != -(ENOSYS as isize) {
        return ret as core::ffi::c_int;
    }

    // SYS_pipe2 不可用，回退手动设置
    if flag & !((O_CLOEXEC | O_NONBLOCK) as core::ffi::c_int) != 0 {
        return -1; /* EINVAL */
    }

    let r = pipe(fd);
    if r != 0 { return r; }

    let fd0 = *fd;
    let fd1 = *fd.add(1);

    if flag & (O_CLOEXEC as core::ffi::c_int) != 0 {
        syscall!(SYS_fcntl, fd0, F_SETFD, FD_CLOEXEC);
        syscall!(SYS_fcntl, fd1, F_SETFD, FD_CLOEXEC);
    }
    if flag & (O_NONBLOCK as core::ffi::c_int) != 0 {
        syscall!(SYS_fcntl, fd0, F_SETFL, O_NONBLOCK);
        syscall!(SYS_fcntl, fd1, F_SETFL, O_NONBLOCK);
    }
    0
}
```

---

## Rust 安全包装（模块内部）

```rust
pub(crate) fn create_pipe2(flags: core::ffi::c_int) -> Result<(core::ffi::c_int, core::ffi::c_int), Error> {
    let mut fds: [core::ffi::c_int; 2] = [-1, -1];
    let ret = unsafe { pipe2(fds.as_mut_ptr(), flags) };
    if ret < 0 {
        Err(Error::last_os_error())
    } else {
        Ok((fds[0], fds[1]))
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  pipe(fd)                                    // 依赖1: pipe 函数 (flag=0 时委托)
  SYS_pipe2                                   // 依赖2: Linux pipe2 系统调用 (原子操作)
  SYS_fcntl + F_SETFD / F_SETFL               // 依赖3: 手动设置描述符标志
Predefined Macros/Crates:
  O_CLOEXEC, O_NONBLOCK, FD_CLOEXEC           // 依赖4: 标志常量

[GUARANTEE]
Exported Interface:
  extern "C" fn pipe2(fd: *mut core::ffi::c_int, flag: core::ffi::c_int) -> core::ffi::c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的 pipe2 符号
Internal Interface:
  pub(crate) fn create_pipe2(flags: core::ffi::c_int) -> Result<(core::ffi::c_int, core::ffi::c_int), Error>;
                                   // 安全 Rust 包装
