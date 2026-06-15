# readlink — Rust 接口归约

## 原始 C 接口
```c
ssize_t readlink(const char *restrict path, char *restrict buf, size_t bufsize);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn readlink(
    path: *const core::ffi::c_char,
    buf: *mut core::ffi::c_char,
    bufsize: usize,
) -> isize;
```

---

## 意图
读取符号链接 `path` 的目标路径，将最多 `bufsize` 字节（不含终止 '\0'）复制到 `buf` 中。注意返回的路径字符串**不**以 '\0' 结尾。

## 前置条件
- `path`: 已存在的符号链接路径
- `buf`: 指向至少 `bufsize` 字节的缓冲区（若 bufsize>0）
- `bufsize`: 缓冲区大小（0 表示只验证符号链接存在而不读取内容）

## 后置条件
- Case 1 成功
  - 返回写入 `buf` 的字节数（不含 '\0'）
  - `buf` 包含目标路径的内容前缀（若 bufsize 小于链接目标长度）
  - 若 bufsize==0：返回 0（musl 特殊处理，防止内核写 NULL）
- Case 2 错误
  - 返回 -1
  - `errno` 设置为 `ENOENT`（不是符号链接）、`EINVAL`（不是符号链接）等

## 不变量
- 当 `bufsize == 0` 时，函数使用私有栈缓冲区保护内核调用，然后返回 0

## 算法
原 C 实现的核心流程：

```
readlink(path, buf, bufsize):
  if !bufsize:
    buf = dummy; bufsize = 1  // 防止内核空指针解引用

  r = __syscall(SYS_readlink / SYS_readlinkat(AT_FDCWD), path, buf, bufsize)

  if buf == dummy && r > 0: r = 0  // bufsize==0 的情况恢复

  return __syscall_ret(r)
```

Rust 中：

```rust
const AT_FDCWD: core::ffi::c_int = -100;

// 方案：实现 bufsize==0 的 dummy 缓冲区保护逻辑
#[inline]
unsafe fn readlink_impl(
    path: *const core::ffi::c_char,
    buf: *mut core::ffi::c_char,
    bufsize: usize,
) -> isize {
    let (buf_ptr, size) = if bufsize == 0 {
        // 使用栈上 dummy 缓冲区防止内核空指针解引用
        let mut dummy: u8 = 0;
        (&raw mut dummy as *mut core::ffi::c_char, 1usize)
    } else {
        (buf, bufsize)
    };
    let r = sys_readlink(path, buf_ptr, size);
    if bufsize == 0 && r > 0 { 0 } else { r }
}
```

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;
use std::io;

// 安全包装：返回写入的字节数（不含 '\0'）
pub(crate) fn readlink_safe(
    path: &CStr,
    buf: &mut [u8],
) -> io::Result<usize> {
    let ret = unsafe {
        readlink_impl(path.as_ptr(), buf.as_mut_ptr() as *mut core::ffi::c_char, buf.len())
    };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret as usize)
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_readlink — Linux 内核系统调用号（部分架构如 x86_64）
  SYS_readlinkat — Linux 内核系统调用号（回退方案）
  AT_FDCWD — 相对于当前工作目录的特殊 fd 值（-100）
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn readlink(
      path: *const core::ffi::c_char,
      buf: *mut core::ffi::c_char,
      bufsize: usize,
  ) -> isize;
                                 // 本模块保证对外提供与 C ABI 兼容的 readlink 符号
Internal Interface:
  pub(crate) fn readlink_safe(
      path: &CStr,
      buf: &mut [u8],
  ) -> io::Result<usize>;
                                 // 安全 Rust 包装
