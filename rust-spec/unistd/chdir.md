# chdir — Rust 接口归约

## 原始 C 接口
```c
int chdir(const char *path);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn chdir(path: *const core::ffi::c_char) -> core::ffi::c_int;
```

---

## 意图
将调用进程的当前工作目录改为 `path` 指定的目录。Rust 侧通过 `extern "C"` 导出符号，内部直接调用 Linux `SYS_chdir` 系统调用（通过内联 `syscall!`）。

## 前置条件
- `path`: 以 NULL 结尾的有效目录路径字符串

## 后置条件
- Case 1 成功: 当前工作目录变更为 `path`，返回 0
- Case 2 错误: 返回 -1，设置 errno（`ENOENT`、`EACCES`、`ENOTDIR`、`ELOOP` 等）

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现为纯系统调用封装：`syscall(SYS_chdir, path)`。Rust 中：

```rust
// 直接调用底层 syscall
#[inline]
unsafe fn sys_chdir(path: *const core::ffi::c_char) -> core::ffi::c_int {
    // arch-specific syscall invocation
    // x86_64: syscall!(SYS_chdir, path)
    // aarch64: svc #0 with x8 = SYS_chdir
}
```

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;

// 安全包装，接受 &CStr 或 &str
pub(crate) fn chdir_safe(path: &CStr) -> io::Result<()> {
    let ret = unsafe { sys_chdir(path.as_ptr()) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_chdir — Linux 内核系统调用号
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn chdir(path: *const core::ffi::c_char) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 chdir 符号
Internal Interface:
  pub(crate) fn chdir_safe(path: &CStr) -> io::Result<()>;
                                 // 安全 Rust 包装
