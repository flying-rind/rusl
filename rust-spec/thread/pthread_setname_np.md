# pthread_setname_np — Rust 接口归约

## 原始 C 接口
```c
int pthread_setname_np(pthread_t thread, const char *name);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn pthread_setname_np(
    thread: pthread_t,
    name: *const core::ffi::c_char
) -> core::ffi::c_int;
```

---

## 意图

设置指定线程的名称（GNU 扩展 `_np` = non-portable）。对线程自身使用 `prctl(PR_SET_NAME)` 直接设置；对其他线程通过写入 `/proc/self/task/<tid>/comm` 设置。rusl 内部对字符串参数使用 `&CStr` 安全视图，文件写入路径使用 safe Rust 系统调用封装。

## 前置条件

- `thread` 是有效的 `pthread_t`
- `name` 为非空指针，指向以 null 结尾、长度不超过 15 字符的字符串

## 后置条件

- Case 1 名称长度 > 15：返回 `ERANGE`，名称不变
- Case 2 `thread == pthread_self()` 且 `prctl(PR_SET_NAME)` 成功：线程名称已更新，返回 `0`
- Case 3 `thread == pthread_self()` 且 `prctl` 失败：返回 `errno`
- Case 4 `thread != pthread_self()`：写入 `/proc/self/task/<tid>/comm`，成功时返回 `0`，失败时返回 `errno`

## 不变量

- 线程名最大长度为 15 字符（Linux 内核限制，16 字节含终止 null）
- 在 `/proc` 文件操作期间禁用线程取消，确保操作原子性

## 算法

对外导出保持 ABI 兼容的 `extern "C"` 函数。内部使用 `core::ffi::CStr` 安全地处理 `name` 字符串参数。

```rust
// extern "C" 函数内部流程
// pthread_setname_np(thread, name):
//   1. 将 name 构造为 &CStr（safe），计算字节长度
//      unsafe { CStr::from_ptr(name) }
//   2. 若长度 > 15（不含 null）→ 返回 ERANGE
//   3. 若 thread == pthread_self()：
//      - 调用 prctl(PR_SET_NAME, name) → 成功返回 0，失败返回 errno
//   4. 否则（其他线程）：
//      a. 格式化路径 "/proc/self/task/{tid}/comm"
//      b. 禁用线程取消
//      c. 以只写方式打开文件
//      d. 写入 name 字节到文件
//      e. 若任意操作失败：记录 errno
//      f. 关闭文件描述符
//      g. 恢复线程取消状态
//      h. 返回结果
```

内部改进要点：
- `name` 参数通过 `unsafe { CStr::from_ptr(name) }` 转换为安全的 `&CStr` 视图
- 长度检查使用 `name_cstr.to_bytes().len()` 替代 C 的 `strnlen`
- 路径格式化使用 Rust 的 `write!` 宏
- 文件 I/O 通过 crate 内部的 syscall 封装执行

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（供 crate 内部使用）
pub(crate) fn set_thread_name(thread: pthread_t, name: &core::ffi::CStr) -> Result<(), core::ffi::c_int>;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr                                    // 依赖1: Rust 安全的 C 字符串视图（替代 strnlen）
  pthread_self()                                     // 依赖2: 获取当前线程标识
  prctl()                                            // 依赖3: Linux 进程控制系统调用
  crate::stdio::snprintf()                           // 依赖4: 格式化路径字符串（或 Rust write! 宏替代）
  pthread_setcancelstate()                           // 依赖5: 控制取消状态
  open() / write() / close()                         // 依赖6: POSIX 文件 I/O 系统调用
  errno                                              // 依赖7: 错误码
Predefined Macros/Traits:
  PR_SET_NAME                                        // 依赖8: prctl 操作码
  O_WRONLY / O_CLOEXEC                               // 依赖9: open 标志

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_setname_np(thread: pthread_t, name: *const core::ffi::c_char) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn set_thread_name(thread: pthread_t, name: &core::ffi::CStr) -> Result<(), core::ffi::c_int>;
