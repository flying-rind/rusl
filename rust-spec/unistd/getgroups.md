# getgroups — Rust 接口归约

## 原始 C 接口
```c
int getgroups(int count, gid_t list[]);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn getgroups(count: core::ffi::c_int, list: *mut core::ffi::c_uint) -> core::ffi::c_int;
```

---

## 意图

获取调用进程的附加组列表（Supplementary Group IDs）。使用 `syscall` 宏（即 `__syscall_ret(__syscall(...))`）确保内核返回的错误能正确转换为 libc 约定。

## 前置条件

- `count`: 如果 `count == 0`，函数返回附加组的数量而不填充 `list`
- `count`: 如果 `count > 0`，必须大于等于附加组的实际数量
- `list` (当 `count > 0` 时): 指向至少 `count` 个 `gid_t` 元素的非空数组

## 后置条件

- **Case 1 `count == 0`**
  - 返回附加组的数量
  - `list` 不会被修改

- **Case 2 成功获取（`count >= 实际组数）**
  - `list[0..n-1]` 被填充为进程的附加组 ID
  - 返回实际附加组数量 `n`（`0 <= n <= NGROUPS_MAX`）

- **Case 3 `count` 太小**
  - 返回 -1
  - `errno` 设置为 `EINVAL`
  - `list` 不会被修改

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现调用 `syscall(SYS_getgroups, count, list)`。Rust 中：

```rust
extern "C" fn getgroups(count: core::ffi::c_int, list: *mut core::ffi::c_uint) -> core::ffi::c_int {
    // 调用 SYS_getgroups 系统调用
    unsafe { syscall_with_ret(SYS_getgroups, count, list) }
}
```

即：
1. 调用 `__syscall(SYS_getgroups, count, list)` 执行内核系统调用
2. 通过 `__syscall_ret()` 将内核返回值转换为 libc 约定（错误时设置 errno 返回 -1）

---

## Rust 安全包装（模块内部）

```rust
// 安全包装，返回 Vec<u32>
// 内部先调用 getgroups(0, null) 获取数量，再分配缓冲区后调用
pub(crate) fn get_supplementary_groups() -> Result<Vec<u32>, Errno> {
    // 1. 先获取数量
    // 2. 分配 Vec
    // 3. 调用获取实际组列表
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_getgroups syscall — Linux 内核系统调用编号 (x86_64: 115, aarch64: 158)
  __syscall_ret — 内核返回值到 libc 错误码转换
Predefined Macros/Crates:
  syscall 模块 (rusl 内部 syscall 封装)

[GUARANTEE]
Exported Interface:
  extern "C" fn getgroups(count: core::ffi::c_int, list: *mut core::ffi::c_uint) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 getgroups 符号
Internal Interface:
  pub(crate) fn get_supplementary_groups() -> Result<Vec<u32>, Errno>;
                                 // 安全 Rust 包装，返回堆分配的组 ID 列表
