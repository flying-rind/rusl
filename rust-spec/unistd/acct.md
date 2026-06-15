# acct — Rust 接口归约

## 原始 C 接口
```c
int acct(const char *filename);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn acct(filename: *const core::ffi::c_char) -> core::ffi::c_int;
```

---

## 意图
启用或禁用进程记账。若 `filename` 非 NULL，内核开始将进程记账信息写入指定文件；若 `filename` 为 NULL，禁用记账。需要 `CAP_SYS_PACCT` 权限。

该调用是 `SYS_acct` 系统调用的薄封装。

## 前置条件
- `filename`: 记账文件路径（NULL 禁用记账），指向以 NUL 结尾的有效 C 字符串
- 需要特权权限（`CAP_SYS_PACCT`）

## 后置条件
- **Case 1 成功**
  - 记账启用（若 `filename` 非 NULL）或禁用（若 `filename` 为 NULL）
  - 返回 0

- **Case 2 错误**
  - 返回 -1
  - `errno` 设置为相应错误码：
    - `EPERM`: 调用进程没有 `CAP_SYS_PACCT` 权限
    - `EACCES`: 对指定文件没有写入权限
    - `ENOSYS`: 内核未编译进程记账支持（`CONFIG_BSD_PROCESS_ACCT`）
    - `ENOENT`: 指定文件的路径不存在
    - `EUSERS`: 没有更多可用的文件描述符/空间

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现调用 `syscall(SYS_acct, filename)` 进行系统调用并转换返回值。Rust 中：

```
acct(filename):
  ret = syscall(SYS_acct, filename)
  if ret < 0:
    errno = -ret
    return -1
  return 0
```

即：
1. 调用 `SYS_acct` 系统调用
2. 通过 `syscall_ret` 将内核返回值转换为 libc 约定（错误时设置 errno 返回 -1）

---

## Rust 安全包装（模块内部）

```rust
/// 启用或禁用进程记账。
/// `filename` 为 `None` 时禁用记账。
pub(crate) fn set_accounting(filename: Option<&core::ffi::CStr>) -> Result<(), Errno> {
    let path_ptr = filename.map(|s| s.as_ptr()).unwrap_or(core::ptr::null());
    unsafe {
        let ret = syscall(SYS_acct, path_ptr);
        if ret < 0 {
            return Err(Errno::from_raw(-ret as i32));
        }
    }
    Ok(())
}
```

---

## 注意事项
- `acct` 是 GNU 扩展（`_GNU_SOURCE`），非 POSIX 标准函数
- 仅在 Linux 上可用，且需要内核启用 `CONFIG_BSD_PROCESS_ACCT` 支持

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_acct (syscall number)           // 依赖1: 底层 acct 系统调用
  syscall_ret 转换逻辑                // 依赖2: 内核返回值到 libc 约定的转换
Predefined Macros/Crates:
  syscall! 宏或等效 no_std 封装      // 依赖3: 系统调用封装

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn acct(filename: *const core::ffi::c_char) -> core::ffi::c_int;
                                       // 本模块保证对外提供与 C ABI 兼容的 acct 符号
Internal Interface:
  pub(crate) fn set_accounting(filename: Option<&core::ffi::CStr>) -> Result<(), Errno>;
                                       // 安全 Rust 包装
