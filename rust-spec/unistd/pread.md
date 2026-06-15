# pread — Rust 接口归约

## 原始 C 接口
```c
ssize_t pread(int fd, void *buf, size_t size, off_t ofs);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn pread(fd: core::ffi::c_int, buf: *mut core::ffi::c_void, size: usize, ofs: i64) -> isize;
```

> **注意**: `off_t` 在 musl 中始终为 64 位（`i64`），与指针宽度无关。`__SYSCALL_LL_PRW` 在 32 位平台上将 64 位 off_t 展开为两个 `long` 参数。

---

## 意图
从文件描述符 `fd` 的指定偏移 `ofs` 处读取至多 `size` 字节到缓冲区 `buf`，文件偏移指针不变。等效于 `lseek(fd, ofs, SEEK_SET)` + `read(fd, buf, size)` + 恢复原偏移的原子操作。

## 前置条件
- `fd`: 有效的已打开文件描述符，必须支持定位操作（`c_int`）
- `buf`: 指向至少 `size` 字节可用内存的非空指针（`*mut c_void`）
- `size`: `[0, SSIZE_MAX]` 范围内的字节数（`usize`）
- `ofs`: 文件中的有效偏移位置（`i64`）

## 后置条件
- **Case 1 成功**: 返回实际读取的字节数（0 表示 EOF），`fd` 的当前文件偏移不变
- **Case 2 错误**: 返回 `-1`，`errno` 设置为 `EBADF`、`ESPIPE`（不可定位 fd）等

## 不变量
无。

## 算法
原 C 实现：
```
pread(fd, buf, size, ofs):
  return syscall_cp(SYS_pread, fd, buf, size, __SYSCALL_LL_PRW(ofs))
```

`__SYSCALL_LL_PRW(ofs)` 在 32 位平台上将 `off_t`（64 位）展开为两个 long 参数（高 32 位在前），在 64 位平台上直接使用 ofs。

Rust 中：

```rust
#[inline]
unsafe fn sys_pread(fd: core::ffi::c_int, buf: *mut core::ffi::c_void, size: usize, ofs: i64) -> isize {
    #[cfg(target_pointer_width = "64")]
    {
        syscall!(SYS_pread, fd, buf, size, ofs) as isize
    }
    #[cfg(target_pointer_width = "32")]
    {
        syscall!(SYS_pread, fd, buf, size, (ofs >> 32) as i32, ofs as i32) as isize
    }
}
```

---

## Rust 安全包装（模块内部）

```rust
pub(crate) fn read_at(fd: core::ffi::c_int, buf: &mut [u8], offset: i64) -> Result<usize, Error> {
    let nread = unsafe { pread(fd, buf.as_mut_ptr() as *mut core::ffi::c_void, buf.len(), offset) };
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
  SYS_pread                                   // 依赖1: Linux 系统调用编号
Predefined Macros/Crates:
  target_pointer_width 编译时条件              // 依赖2: 32/64 位偏移适配
  syscall! 宏                                 // 依赖3: 系统调用入口

[GUARANTEE]
Exported Interface:
  extern "C" fn pread(fd: core::ffi::c_int, buf: *mut core::ffi::c_void, size: usize, ofs: i64) -> isize;
                                   // 本模块保证对外提供与 C ABI 兼容的 pread 符号
Internal Interface:
  pub(crate) fn read_at(fd: core::ffi::c_int, buf: &mut [u8], offset: i64) -> Result<usize, Error>;
                                   // 安全 Rust 包装
