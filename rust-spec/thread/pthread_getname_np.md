# pthread_getname_np — Rust 接口归约

## 原始 C 接口
```c
int pthread_getname_np(pthread_t thread, char *name, size_t len);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn pthread_getname_np(
    thread: pthread_t,
    name: *mut core::ffi::c_char,
    len: usize
) -> core::ffi::c_int;
```

---

## 意图

获取指定线程的名称（GNU 扩展 `_np` = non-portable）。对线程自身使用 `prctl(PR_GET_NAME)` 直接获取；对其他线程通过读取 `/proc/self/task/<tid>/comm` 获取。rusl 内部可用 Safe Rust 实现文件读取路径，仅系统调用层面使用 `unsafe`。

## 前置条件

- `thread` 是有效的 `pthread_t`
- `name` 为非空指针，指向可写缓冲区
- `len >= 16`（线程名最大 16 字符含终止 null）

## 后置条件

- Case 1 `len < 16`：返回 `ERANGE`，不修改 `name` 缓冲区
- Case 2 `thread == pthread_self()` 且 `prctl(PR_GET_NAME)` 成功：`name` 缓冲区被填充为当前线程名（null 结尾），返回 `0`
- Case 3 `thread == pthread_self()` 且 `prctl` 失败：返回 `errno`
- Case 4 `thread != pthread_self()`：读取 `/proc/self/task/<tid>/comm`，成功时 `name` 包含线程名（去除末尾换行符），返回 `0`；失败时返回对应的 `errno`

## 不变量

- 在 `/proc` 文件操作期间禁用线程取消，确保操作原子性
- 读取成功后总是去除末尾换行符（`/proc/.../comm` 以换行结尾）

## 算法

对外导出保持 ABI 兼容的 `extern "C"` 函数，内部读写 `/proc` 文件时使用 safe Rust 的系统调用包装。

```rust
// extern "C" 函数内部流程
// pthread_getname_np(thread, name, len):
//   1. 若 len < 16 → 返回 ERANGE
//   2. 若 thread == pthread_self()：
//      - 调用 prctl(PR_GET_NAME, name) → 成功返回 0，失败返回 errno
//   3. 否则（其他线程）：
//      a. 格式化路径 "/proc/self/task/{tid}/comm"
//      b. 禁用线程取消
//      c. 以只读方式打开文件
//      d. 读取文件内容到 name 缓冲区
//      e. 若读取成功，将末尾换行符替换为 '\0'
//      f. 若任意操作失败：记录 errno
//      g. 关闭文件描述符
//      h. 恢复线程取消状态
//      i. 返回结果
```

内部改进要点：
- 路径格式化使用 Rust 的 `format!` / `write!` 宏替代 `snprintf`
- 文件 I/O 通过 crate 内部的 syscall 封装执行，仅在 syscall 调用处使用 `unsafe`
- 换行符去除逻辑使用 Rust 的切片操作

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（供 crate 内部使用）
pub(crate) fn get_thread_name(thread: pthread_t, buf: &mut [u8]) -> Result<(), core::ffi::c_int>;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  pthread_self()                                     // 依赖1: 获取当前线程标识
  prctl()                                            // 依赖2: Linux 进程控制系统调用
  crate::stdio::snprintf()                           // 依赖3: 格式化路径字符串（或 Rust format! 宏替代）
  pthread_setcancelstate()                           // 依赖4: 控制取消状态
  open() / read() / close()                          // 依赖5: POSIX 文件 I/O 系统调用
  errno                                              // 依赖6: 错误码
Predefined Macros/Traits:
  PR_GET_NAME                                        // 依赖7: prctl 操作码（来自 <sys/prctl.h>）
  O_RDONLY / O_CLOEXEC                               // 依赖8: open 标志

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_getname_np(thread: pthread_t, name: *mut core::ffi::c_char, len: usize) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn get_thread_name(thread: pthread_t, buf: &mut [u8]) -> Result<(), core::ffi::c_int>;
