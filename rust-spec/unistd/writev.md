# writev — Rust 接口归约

## 原始 C 接口
```c
ssize_t writev(int fd, const struct iovec *iov, int count);
```

---

## Rust 外部 ABI 接口

```rust
use core::ffi::{c_int, c_void};

// iovec 结构体定义（需与 C ABI 兼容）
#[repr(C)]
struct Iovec {
    iov_base: *mut c_void,
    iov_len: usize,
}

// 与 C ABI 兼容的底层导出函数
extern "C" fn writev(fd: c_int, iov: *const Iovec, count: c_int) -> isize;
```

---

## 意图
将多个不连续缓冲区（`iov` 数组）的数据原子性地写入文件描述符 `fd`。等效于单次 `write` 调用但避免数据拼接拷贝。

## 前置条件
- `fd`: 有效可写文件描述符（`c_int`）
- `iov`: 指向 `count` 个 `Iovec` 的非空指针
- `count`: `[0, IOV_MAX]` 范围内

## 后置条件
- **Case 1 成功**: 返回实际写入字节数
- **Case 2 错误**: 返回 `-1`，`errno` 设置为对应错误码

## 不变量
无。

## 算法
原 C 实现：`syscall_cp(SYS_writev, fd, iov, count)`。

Rust 中：

```rust
#[inline]
unsafe fn sys_writev(fd: c_int, iov: *const Iovec, count: c_int) -> isize {
    // arch-specific syscall: SYS_writev
    syscall!(SYS_writev, fd, iov, count) as isize
}
```

---

## Rust 安全包装（模块内部）

```rust
pub(crate) fn write_vectored(fd: c_int, bufs: &[IoSlice]) -> Result<usize, Error> {
    let iov: Vec<Iovec> = bufs.iter().map(|b| Iovec {
        iov_base: b.as_ptr() as *mut c_void,
        iov_len: b.len(),
    }).collect();
    let nwritten = unsafe { writev(fd, iov.as_ptr(), iov.len() as c_int) };
    if nwritten < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(nwritten as usize)
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  struct iovec (即 #[repr(C)] Iovec)           // 依赖1: 散布读写向量结构
  SYS_writev                                  // 依赖2: Linux 系统调用编号
Predefined Macros/Crates:
  syscall! 宏                                 // 依赖3: 系统调用入口

[GUARANTEE]
Exported Interface:
  #[repr(C)]
  struct Iovec { iov_base: *mut c_void, iov_len: usize }
                                   // 保证与 C struct iovec 内存布局一致
  extern "C" fn writev(fd: c_int, iov: *const Iovec, count: c_int) -> isize;
                                   // 本模块保证对外提供与 C ABI 兼容的 writev 符号
Internal Interface:
  pub(crate) fn write_vectored(fd: c_int, bufs: &[IoSlice]) -> Result<usize, Error>;
                                   // 安全 Rust 包装
