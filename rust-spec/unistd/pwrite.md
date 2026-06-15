# pwrite — Rust 接口归约

## 原始 C 接口
```c
ssize_t pwrite(int fd, const void *buf, size_t size, off_t ofs);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn pwrite(fd: core::ffi::c_int, buf: *const core::ffi::c_void, size: usize, ofs: i64) -> isize;
```

---

## 意图
向文件描述符 `fd` 的指定偏移 `ofs` 处写入至多 `size` 字节，文件偏移指针不变。musl 优先使用新的 `pwritev2` 系统调用（支持 `RWF_NOAPPEND` 标志），不支持时回退到传统 `pwrite`。

## 前置条件
- `fd`: 有效的已打开文件描述符（`c_int`）
- `buf`: 指向至少 `size` 字节数据的非空指针（`*const c_void`）
- `size`: `[0, SSIZE_MAX]` 范围内的字节数（`usize`）
- `ofs`: 文件中的有效偏移位置（`i64`）

## 后置条件
- **Case 1 成功**: 返回实际写入的字节数，`fd` 的当前文件偏移不变；即使 fd 以 O_APPEND 打开，也在 `ofs` 处写入（pwritev2 语义）
- **Case 2 fd 以 O_APPEND 打开且 pwritev2 不可用**: 返回 `-1`，`errno` 设置为 `EOPNOTSUPP`
- **Case 3 其他错误**: 返回 `-1`，`errno` 设置为对应错误码

## 不变量
无。

## 算法
原 C 实现：
```
pwrite(fd, buf, size, ofs):
  if ofs == -1: ofs--                               // 将 ofs==-1 变为 -2

  // 优先使用 pwritev2，支持 RWF_NOAPPEND
  r = __syscall_cp(SYS_pwritev2, fd, &iovec(1), 1, ofs_lo, ofs_hi, RWF_NOAPPEND)
  if r != -EOPNOTSUPP && r != -ENOSYS: return __syscall_ret(r)

  // pwritev2 不可用，检查 O_APPEND
  if fcntl(fd, F_GETFL) & O_APPEND: return -EOPNOTSUPP

  // 回退到传统 pwrite
  return syscall_cp(SYS_pwrite, fd, buf, size, __SYSCALL_LL_PRW(ofs))
```

Rust 中：

```rust
#[repr(C)]
struct Iovec { iov_base: *mut core::ffi::c_void, iov_len: usize }

#[inline]
unsafe fn sys_pwrite(fd: core::ffi::c_int, buf: *const core::ffi::c_void, size: usize, ofs: i64) -> isize {
    let ofs = if ofs == -1 { ofs - 1 } else { ofs };  // ofs==-1 → -2

    // 优先使用 pwritev2
    #[cfg(target_pointer_width = "64")]
    let r = syscall!(SYS_pwritev2, fd, Iovec { iov_base: buf as *mut _, iov_len: size },
                     1, ofs, RWF_NOAPPEND);
    #[cfg(target_pointer_width = "32")]
    let r = syscall!(SYS_pwritev2, fd, Iovec { iov_base: buf as *mut _, iov_len: size },
                     1, (ofs >> 32) as i32, ofs as i32, RWF_NOAPPEND);

    if r != -(EOPNOTSUPP as isize) && r != -(ENOSYS as isize) {
        return r as isize;
    }

    // 回退路径
    let flags = syscall!(SYS_fcntl, fd, F_GETFL);
    if flags >= 0 && (flags as core::ffi::c_int) & O_APPEND != 0 {
        return -(EOPNOTSUPP as isize);  // 设置 errno = EOPNOTSUPP
    }

    #[cfg(target_pointer_width = "64")]
    { syscall!(SYS_pwrite, fd, buf, size, ofs) as isize }
    #[cfg(target_pointer_width = "32")]
    { syscall!(SYS_pwrite, fd, buf, size, (ofs >> 32) as i32, ofs as i32) as isize }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_pwritev2 (Linux 4.6+)                   // 依赖1: 优先使用的系统调用（支持 RWF_NOAPPEND）
  SYS_pwrite                                  // 依赖2: 传统系统调用（回退）
  SYS_fcntl + F_GETFL + O_APPEND              // 依赖3: 检查追加模式
  struct iovec (即 #[repr(C)] Iovec)           // 依赖4: 散布读写向量
Predefined Macros/Crates:
  RWF_NOAPPEND 标志                           // 依赖5: pwritev2 标志
  target_pointer_width                         // 依赖6: 32/64 位偏移适配

[GUARANTEE]
Exported Interface:
  extern "C" fn pwrite(fd: core::ffi::c_int, buf: *const core::ffi::c_void, size: usize, ofs: i64) -> isize;
                                   // 本模块保证对外提供与 C ABI 兼容的 pwrite 符号
Internal Interface:
  (无额外内部接口)
