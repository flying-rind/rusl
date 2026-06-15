# pipe — Rust 接口归约

## 原始 C 接口
```c
int pipe(int fd[2]);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn pipe(fd: *mut core::ffi::c_int) -> core::ffi::c_int;
```

> **注意**: C 中 `int fd[2]` 等价于 `int *fd`。数组参数在 ABI 层面退化为指针。

---

## 意图
创建一对单向管道文件描述符：`fd[0]` 用于读取，`fd[1]` 用于写入。写入 `fd[1]` 的数据可以通过 `fd[0]` 读取，实现内核缓冲的单向数据流。

## 前置条件
- `fd`: 指向至少 2 个 `c_int` 空间的有效非空指针

## 后置条件
- **Case 1 成功**: `fd[0]` 设置为管道读端，`fd[1]` 设置为管道写端，返回 `0`
- **Case 2 失败**: `fd` 数组内容不变，返回 `-1`，`errno` 设置为 `EMFILE` 或 `ENFILE`

## 不变量
无。

## 算法
原 C 实现：
```
pipe(fd):
  #ifdef SYS_pipe:
    return syscall(SYS_pipe, fd)       // 直接使用 pipe 系统调用
  #else:
    return syscall(SYS_pipe2, fd, 0)   // 回退到 pipe2(fd, 0)
  #endif
```

注意：某些架构（mips、sh）有手写汇编实现的 `pipe` 函数，因为内核 pipe 返回两个值，需要特殊处理。

Rust 中：

```rust
#[inline]
unsafe fn sys_pipe(fd: *mut core::ffi::c_int) -> core::ffi::c_int {
    #[cfg(any(target_arch = "mips", target_arch = "mips64", target_arch = "sh"))]
    {
        // 特殊架构：内核 pipe 返回两个值 (r0=fd[0], r1=fd[1])
        // 需要架构特定的汇编实现
        arch_specific_pipe_syscall(fd)
    }
    #[cfg(not(any(target_arch = "mips", target_arch = "mips64", target_arch = "sh")))]
    {
        #[cfg(has_SYS_pipe)]
        { syscall!(SYS_pipe, fd) as core::ffi::c_int }
        #[cfg(not(has_SYS_pipe))]
        { syscall!(SYS_pipe2, fd, 0) as core::ffi::c_int }
    }
}
```

---

## Rust 安全包装（模块内部）

```rust
pub(crate) fn create_pipe() -> Result<(core::ffi::c_int, core::ffi::c_int), Error> {
    let mut fds: [core::ffi::c_int; 2] = [-1, -1];
    let ret = unsafe { pipe(fds.as_mut_ptr()) };
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
  SYS_pipe  (部分架构)                        // 依赖1: Linux pipe 系统调用
  SYS_pipe2 (部分架构)                        // 依赖2: Linux pipe2 系统调用 (flags=0 等效 pipe)
Predefined Macros/Crates:
  syscall! 宏                                 // 依赖3: 系统调用入口
  target_arch 编译时条件                      // 依赖4: 区分架构 (mips/sh 需特殊处理)

[GUARANTEE]
Exported Interface:
  extern "C" fn pipe(fd: *mut core::ffi::c_int) -> core::ffi::c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的 pipe 符号
Internal Interface:
  pub(crate) fn create_pipe() -> Result<(core::ffi::c_int, core::ffi::c_int), Error>;
                                   // 安全 Rust 包装，返回读/写端 fd
