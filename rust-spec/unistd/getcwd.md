# getcwd — Rust 接口归约

## 原始 C 接口
```c
char *getcwd(char *buf, size_t size);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn getcwd(buf: *mut core::ffi::c_char, size: usize) -> *mut core::ffi::c_char;
```

---

## 意图
获取调用进程的当前工作目录绝对路径名。

- 若 `buf` 非 NULL：将绝对路径名复制到 `buf`（长度最多 `size` 字节）
- 若 `buf` 为 NULL：动态分配缓冲区（大小至少 `PATH_MAX`），调用者负责 `free()`

## 前置条件
- `buf`: NULL（要求动态分配）或指向至少 `size` 字节可用内存的指针
- `size`: 若 `buf` 非 NULL 则必须 > 0 且 >= 路径名长度（含 '\0'）

## 后置条件
- Case 1 成功
  - 返回指向绝对路径名（以 '/' 开头）的指针
  - 若 `buf` 为 NULL：返回的指针指向动态分配的内存（调用者必须 `free()`）
  - 若 `buf` 非 NULL：返回值 == `buf`
- Case 2 `buf` 非 NULL 但 `size` == 0
  - 返回 NULL，`errno` 设置为 `EINVAL`
- Case 3 系统调用失败或返回结果不以 '/' 开头
  - 返回 NULL，`errno` 设置为 `ENOENT` 或 syscall 返回的错误码

## 不变量
- 成功返回的路径始终以 '/' 开头（绝对路径保证）

## 算法
原 C 实现的核心流程：

```
getcwd(buf, size):
  if !buf:
    buf = tmp[PATH_MAX]
    size = PATH_MAX
  else if !size:
    errno = EINVAL; return NULL

  ret = syscall(SYS_getcwd, buf, size)
  if ret < 0: return NULL

  if ret == 0 || buf[0] != '/':
    errno = ENOENT; return NULL

  return (buf == tmp) ? strdup(buf) : buf
```

Rust 中：

```rust
const PATH_MAX: usize = 4096;

// 删除 ptr::eq 判断，直接用 bool 标记 buf 来源
// 方案：直接 syscall 封装，内部实现 buf==NULL 的堆分配逻辑
#[inline]
unsafe fn sys_getcwd(buf: *mut core::ffi::c_char, size: usize) -> isize { /* ... */ }

// 内部函数：从 CStr 复制分配新 CString
fn internal_strdup(s: &core::ffi::CStr) -> *mut core::ffi::c_char { /* ... */ }
```

关于 buf==NULL 时的内存分配：原 C 使用 `strdup` 从栈缓冲区复制到堆。Rust 中可使用全局分配器 `alloc` crate 实现等价逻辑。

---

## Rust 安全包装（模块内部）

```rust
use alloc::ffi::CString;
use core::ffi::CStr;
use std::io;

// 安全包装：总是返回动态分配的 CString
pub(crate) fn getcwd_owned() -> io::Result<CString> {
    // 栈上分配 PATH_MAX 大小的缓冲区
    // 调用 sys_getcwd 获取路径
    // 构造 CString 返回
    todo!()
}

// 安全包装：写入预分配缓冲区
pub(crate) fn getcwd_into(buf: &mut [u8]) -> io::Result<&CStr> {
    todo!()
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_getcwd — Linux 内核系统调用号
  PATH_MAX — <limits.h> 最大路径长度（通常 4096）
  alloc::ffi::CString / alloc::alloc — 动态内存分配（替代原 C 的 strdup）
Predefined Macros/Crates:
  alloc crate — no_std 环境下的动态内存分配
  core::ffi::CStr — C 字符串类型

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn getcwd(buf: *mut core::ffi::c_char, size: usize) -> *mut core::ffi::c_char;
                                 // 本模块保证对外提供与 C ABI 兼容的 getcwd 符号
Internal Interface:
  pub(crate) fn getcwd_owned() -> io::Result<CString>;
                                 // 安全 Rust 包装：返回动态分配的 CString
  pub(crate) fn getcwd_into(buf: &mut [u8]) -> io::Result<&CStr>;
                                 // 安全 Rust 包装：写入预分配缓冲区
