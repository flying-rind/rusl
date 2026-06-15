# truncate — Rust 接口归约

## 原始 C 接口
```c
int truncate(const char *path, off_t length);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn truncate(path: *const core::ffi::c_char, length: i64) -> core::ffi::c_int;
```

> **注意**: `off_t` 在 musl 中始终为 64 位（`i64`）。`__SYSCALL_LL_O` 在 32 位平台上将 64 位 length 展开为两个 `long` 参数。`path` 为 `*const c_char` 表示 NULL 结尾的字符串。

---

## 意图
将 `path` 指定的普通文件截断为精确 `length` 字节。与 `ftruncate` 类似，但通过路径名操作。

## 前置条件
- `path`: 指向已存在普通文件路径的非空指针（`*const c_char`，NULL 结尾字符串）
- `length`: 非负整数（`i64`）

## 后置条件
- **Case 1 成功**: 文件大小变为 `length`，返回 `0`
- **Case 2 错误**: 返回 `-1`，`errno` 设置为对应错误码

## 不变量
无。

## 算法
原 C 实现：`syscall(SYS_truncate, path, __SYSCALL_LL_O(length))`。

Rust 中：

```rust
#[inline]
unsafe fn sys_truncate(path: *const core::ffi::c_char, length: i64) -> core::ffi::c_int {
    #[cfg(target_pointer_width = "64")]
    {
        syscall!(SYS_truncate, path, length) as core::ffi::c_int
    }
    #[cfg(target_pointer_width = "32")]
    {
        syscall!(SYS_truncate, path, (length >> 32) as i32, length as i32) as core::ffi::c_int
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_truncate                                // 依赖1: Linux 系统调用编号
Predefined Macros/Crates:
  target_pointer_width 编译时条件              // 依赖2: 32/64 位长度适配
  syscall! 宏                                 // 依赖3: 系统调用入口

[GUARANTEE]
Exported Interface:
  extern "C" fn truncate(path: *const core::ffi::c_char, length: i64) -> core::ffi::c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的 truncate 符号
Internal Interface:
  (无额外内部接口)
