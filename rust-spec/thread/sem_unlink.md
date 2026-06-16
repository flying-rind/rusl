# sem_unlink — Rust 接口归约

## 原始 C 接口
```c
int sem_unlink(const char *name);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn sem_unlink(name: *const core::ffi::c_char) -> core::ffi::c_int;
```

---

## 意图

从系统中移除有名信号量。直接委托给 `shm_unlink()`，删除 `/dev/shm` 中对应的共享内存文件。rusl 内部通过系统调用封装实现，仅在一处使用 `unsafe` 调用 `shm_unlink`。

## 前置条件

- `name` 为非空指针，指向 '/' 开头的信号量名称

## 后置条件

- Case 1 成功：名称从系统中移除，返回 `0`。已打开该信号量的线程仍可继续使用，但不再能通过 `sem_open` 打开
- Case 2 失败：返回 `-1`，`errno` 设置对应错误

## 不变量

- 无额外逻辑，纯粹的一对一委托给 `shm_unlink`
- 信号量名称被移除后不影响已存在的 mmap 映射

## 算法

对外导出保持 ABI 兼容的 `extern "C"` 函数。内部纯委托。

```rust
// extern "C" 函数内部流程
// sem_unlink(name):
//   调用 shm_unlink(name) → 返回其结果（0 或 -1）
```

内部改进要点：
- `name` 参数通过 `unsafe { CStr::from_ptr(name) }` 转换为安全视图后再传递给下层系统调用
- 底层 `shm_unlink` 系统调用封装在单个 `unsafe` 块中

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装
pub(crate) fn sem_unlink_inner(name: &core::ffi::CStr) -> Result<(), core::ffi::c_int>;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  shm_unlink()                                       // 依赖1: POSIX 共享内存卸载系统调用
  core::ffi::CStr                                    // 依赖2: Rust 安全的 C 字符串视图
Predefined Macros/Traits:
  (无)                                                // 纯委托

[GUARANTEE]
Exported Interface:
  extern "C" fn sem_unlink(name: *const core::ffi::c_char) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn sem_unlink_inner(name: &core::ffi::CStr) -> Result<(), core::ffi::c_int>;
