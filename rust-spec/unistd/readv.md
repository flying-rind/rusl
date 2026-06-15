# readv — Rust 接口归约

## 原始 C 接口
```c
ssize_t readv(int fd, const struct iovec *iov, int count);
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
extern "C" fn readv(fd: c_int, iov: *const Iovec, count: c_int) -> isize;
```

---

## 意图
从文件描述符 `fd` 读取数据到多个不连续的缓冲区（`iov` 数组），原子操作，等效于单次 `read` 到拼接缓冲区但无需数据拷贝。

## 前置条件
- `fd`: 有效可读文件描述符（`c_int`）
- `iov`: 指向 `count` 个 `Iovec` 的非空指针，每个元素含 `iov_base`（缓冲区指针）和 `iov_len`（缓冲区大小）
- `count`: `[0, IOV_MAX]` 范围内的向量数量

## 后置条件
- **Case 1 成功**: 返回实际读取字节数（可能小于总缓冲区大小）
- **Case 2 错误**: 返回 `-1`，`errno` 设置为对应错误码

## 不变量
无。

## 算法
原 C 实现：`syscall_cp(SYS_readv, fd, iov, count)`。

Rust 中：

```rust
#[inline]
unsafe fn sys_readv(fd: c_int, iov: *const Iovec, count: c_int) -> isize {
    // arch-specific syscall: SYS_readv
    syscall!(SYS_readv, fd, iov, count) as isize
}
```

---

## Rust 安全包装（模块内部）

```rust
pub(crate) fn read_vectored(fd: c_int, bufs: &mut [IoSliceMut]) -> Result<usize, Error> {
    let iov: Vec<Iovec> = bufs.iter().map(|b| Iovec {
        iov_base: b.as_mut_ptr() as *mut c_void,
        iov_len: b.len(),
    }).collect();
    let nread = unsafe { readv(fd, iov.as_ptr(), iov.len() as c_int) };
    if nread < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(nread as usize)
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  struct iovec (即 #[repr(C)] Iovec)           // 依赖1: 散布读写向量结构
  SYS_readv                                   // 依赖2: Linux 系统调用编号
Predefined Macros/Crates:
  syscall! 宏                                 // 依赖3: 系统调用入口

[GUARANTEE]
Exported Interface:
  #[repr(C)]
  struct Iovec { iov_base: *mut c_void, iov_len: usize }
                                   // 保证与 C struct iovec 内存布局一致
  extern "C" fn readv(fd: c_int, iov: *const Iovec, count: c_int) -> isize;
                                   // 本模块保证对外提供与 C ABI 兼容的 readv 符号
Internal Interface:
  pub(crate) fn read_vectored(fd: c_int, bufs: &mut [IoSliceMut]) -> Result<usize, Error>;
                                   // 安全 Rust 包装
