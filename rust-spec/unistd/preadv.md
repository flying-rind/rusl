# preadv — Rust 接口归约

## 原始 C 接口
```c
ssize_t preadv(int fd, const struct iovec *iov, int count, off_t ofs);
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
extern "C" fn preadv(fd: c_int, iov: *const Iovec, count: c_int, ofs: i64) -> isize;
```

---

## 意图
从文件描述符 `fd` 的指定偏移 `ofs` 处分散读取数据到多个缓冲区，不改变当前文件偏移。等效于 `lseek + readv + lseek` 的原子操作。

## 前置条件
- `fd`: 有效、支持定位的文件描述符（`c_int`）
- `iov`: 指向 `count` 个 `Iovec` 的非空指针
- `count`: `[0, IOV_MAX]` 范围内
- `ofs`: 文件偏移（`i64`）

## 后置条件
- **Case 1 成功**: 返回实际读取字节数，文件偏移不变
- **Case 2 错误**: 返回 `-1`，`errno` 设置为对应错误码

## 不变量
无。

## 算法
原 C 实现：
```
preadv(fd, iov, count, ofs):
  return syscall_cp(SYS_preadv, fd, iov, count, (long)(ofs), (long)(ofs>>32))
```

32 位平台上 `off_t`（64 位）被拆分为两个 `long` 参数（低 32 位在前，高 32 位在后）。

Rust 中：

```rust
#[inline]
unsafe fn sys_preadv(fd: c_int, iov: *const Iovec, count: c_int, ofs: i64) -> isize {
    #[cfg(target_pointer_width = "64")]
    {
        syscall!(SYS_preadv, fd, iov, count, ofs) as isize
    }
    #[cfg(target_pointer_width = "32")]
    {
        syscall!(SYS_preadv, fd, iov, count, ofs as i32, (ofs >> 32) as i32) as isize
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_preadv                                  // 依赖1: Linux 系统调用编号
  struct iovec (即 #[repr(C)] Iovec)           // 依赖2: 散布读写向量结构
Predefined Macros/Crates:
  target_pointer_width                         // 依赖3: 32/64 位偏移适配
  syscall! 宏                                 // 依赖4: 系统调用入口

[GUARANTEE]
Exported Interface:
  extern "C" fn preadv(fd: c_int, iov: *const Iovec, count: c_int, ofs: i64) -> isize;
                                   // 本模块保证对外提供与 C ABI 兼容的 preadv 符号
Internal Interface:
  (无额外内部接口)
