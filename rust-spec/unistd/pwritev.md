# pwritev — Rust 接口归约

## 原始 C 接口
```c
ssize_t pwritev(int fd, const struct iovec *iov, int count, off_t ofs);
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
extern "C" fn pwritev(fd: c_int, iov: *const Iovec, count: c_int, ofs: i64) -> isize;
```

---

## 意图
将多个缓冲区的数据原子性地写入文件描述符 `fd` 的指定偏移 `ofs` 处，文件偏移不变。优先使用 `pwritev2` 系统调用以支持 `RWF_NOAPPEND`，确保即使 fd 以 O_APPEND 打开也能在指定偏移写入。

## 前置条件
- `fd`: 有效可写文件描述符（`c_int`）
- `iov`: 指向 `count` 个 `Iovec` 的非空指针
- `count`: `[0, IOV_MAX]` 范围内
- `ofs`: 文件偏移（-1 会被调整为 -2 以区分特殊语义）（`i64`）

## 后置条件
- **Case 1 成功**: 返回实际写入字节数，文件偏移不变
- **Case 2 fd 以 O_APPEND 打开且 pwritev2 不可用**: 返回 `-1`，`errno` 设置为 `EOPNOTSUPP`
- **Case 3 其他错误**: 返回 `-1`，`errno` 设置为对应错误码

## 不变量
无。

## 算法
原 C 实现：
```
pwritev(fd, iov, count, ofs):
  if ofs == -1: ofs--                                                    // ofs==-1 → -2
  r = __syscall_cp(SYS_pwritev2, fd, iov, count, ofs_lo, ofs_hi, RWF_NOAPPEND)
  if r != -EOPNOTSUPP && r != -ENOSYS: return __syscall_ret(r)
  if fcntl(fd, F_GETFL) & O_APPEND: return __syscall_ret(-EOPNOTSUPP)
  return syscall_cp(SYS_pwritev, fd, iov, count, ofs_lo, ofs_hi)
```

Rust 中：

```rust
#[inline]
unsafe fn sys_pwritev(fd: c_int, iov: *const Iovec, count: c_int, ofs: i64) -> isize {
    let ofs = if ofs == -1 { ofs - 1 } else { ofs };  // ofs==-1 → -2

    // 优先使用 pwritev2
    #[cfg(target_pointer_width = "64")]
    let r = syscall!(SYS_pwritev2, fd, iov, count, ofs, RWF_NOAPPEND);
    #[cfg(target_pointer_width = "32")]
    let r = syscall!(SYS_pwritev2, fd, iov, count, ofs as i32, (ofs >> 32) as i32, RWF_NOAPPEND);

    if r != -(EOPNOTSUPP as isize) && r != -(ENOSYS as isize) {
        return r as isize;
    }

    // 回退路径
    let flags = syscall!(SYS_fcntl, fd, F_GETFL);
    if flags >= 0 && (flags as c_int) & O_APPEND != 0 {
        return -(EOPNOTSUPP as isize);
    }

    #[cfg(target_pointer_width = "64")]
    { syscall!(SYS_pwritev, fd, iov, count, ofs) as isize }
    #[cfg(target_pointer_width = "32")]
    { syscall!(SYS_pwritev, fd, iov, count, ofs as i32, (ofs >> 32) as i32) as isize }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_pwritev2 (Linux 4.6+)                   // 依赖1: 优先使用的系统调用
  SYS_pwritev                                 // 依赖2: 传统系统调用（回退）
  SYS_fcntl + F_GETFL + O_APPEND              // 依赖3: 检查追加模式
  struct iovec (即 #[repr(C)] Iovec)           // 依赖4: 散布读写向量结构
Predefined Macros/Crates:
  RWF_NOAPPEND 标志                           // 依赖5: pwritev2 标志
  target_pointer_width                         // 依赖6: 32/64 位偏移适配

[GUARANTEE]
Exported Interface:
  extern "C" fn pwritev(fd: c_int, iov: *const Iovec, count: c_int, ofs: i64) -> isize;
                                   // 本模块保证对外提供与 C ABI 兼容的 pwritev 符号
Internal Interface:
  (无额外内部接口)
