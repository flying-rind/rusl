# ttyname_r — Rust 接口归约

## 原始 C 接口
```c
int ttyname_r(int fd, char *name, size_t size);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn ttyname_r(fd: core::ffi::c_int, name: *mut core::ffi::c_char, size: usize) -> core::ffi::c_int;
```

---

## 意图

获取文件描述符 `fd` 所关联的终端设备路径名，写入用户提供的缓冲区 `name`。实现过程：
1. 首先通过 `isatty` 确认 `fd` 是否为终端
2. 通过 `/proc/self/fd/<fd>` 符号链接获取内核记录的路径
3. 读取符号链接目标到 `name` 缓冲区
4. 通过 `stat`/`fstat` 验证设备 ID 和 inode 一致性，防止 `/proc` 挂载点下路径伪造（TOCTOU 保护）

## 前置条件

- `fd`: 有效的已打开文件描述符
- `name`: 指向至少 `size` 字节可用内存的非空指针
- `size`: 缓冲区大小，`> 0`

## 后置条件

- **Case 1 成功**
  - `name` 缓冲区被写入以 null 结尾的终端设备路径名（如 `/dev/pts/0`）
  - 终端设备通过 stat/fstat 验证（设备 ID 和 inode 一致）
  - 返回 0

- **Case 2 `fd` 不是终端**
  - 返回 `errno`（由 `isatty` 设置，如 `ENOTTY`、`EBADF`）

- **Case 3 读取符号链接失败**
  - 返回 `errno`（如 `ENOENT`、`EACCES`）

- **Case 4 缓冲区太小**
  - 返回 `ERANGE`
  - `name` 内容未定义

- **Case 5 设备验证失败（TOCTOU 保护）**
  - `stat` 或 `fstat` 失败：返回 `errno`
  - 设备 ID 或 inode 不匹配：返回 `ENODEV`

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现逐步执行 isatty 检测、/proc/self/fd 符号链接读取、stat/fstat 验证。Rust 中：

```rust
extern "C" fn ttyname_r(fd: core::ffi::c_int, name: *mut core::ffi::c_char, size: usize) -> core::ffi::c_int {
    // 1. 检查是否为终端
    if unsafe { isatty(fd) } == 0 {
        return get_errno();
    }
    // 2. 构造 /proc/self/fd/<fd> 路径
    let proc_path = format_procfd_path(fd);
    // 3. 读取符号链接目标
    let len = unsafe { readlink_str(&proc_path, name, size) };
    if len < 0 { return get_errno(); }
    if len as usize == size { return ERANGE; }
    // 4. 添加 null 终止符
    unsafe { *name.add(len as usize) = 0; }
    // 5. 通过 stat/fstat 验证设备 ID 和 inode 一致性
    if stat_verify(name, fd) != 0 {
        return ENODEV;
    }
    0
}
```

即：
1. 若 `isatty(fd)` 为假（返回 0），直接返回 errno
2. 构造 `/proc/self/fd/<fd>` 路径字符串
3. 调用 `readlink` 读取符号链接目标
4. 若 `readlink` 失败，返回 errno
5. 若返回长度等于缓冲区大小，返回 `ERANGE`（无空间放 null 终止符）
6. 在路径末尾添加 null 终止符
7. 用 `stat` 获取设备文件的设备 ID/inode，用 `fstat` 获取 fd 的设备 ID/inode
8. 若两者不匹配，返回 `ENODEV`
9. 返回 0 表示成功

---

## 安全说明

当前实现存在已知缺陷：`readlink` 返回路径后、`stat`/`fstat` 验证前存在竞争窗口。在此期间终端可能被解除分配并重新分配给其他用户，导致信息泄露。此问题在 POSIX 标准中有记录。

---

## Rust 安全包装（模块内部）

```rust
/// 安全包装，返回 String
/// 内部使用堆分配的 String，自动管理缓冲区大小
pub(crate) fn tty_name_for_fd(fd: RawFd) -> Result<String, Errno> {
    // 1. 分配足够大的缓冲区
    // 2. 调用底层 ttyname_r
    // 3. 将结果转为 Rust String
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  isatty — 终端检测函数
  __procfdname — 构造 /proc/self/fd/<fd> 路径字符串（内部函数）
  readlink — 系统调用，读取符号链接目标
  stat — 系统调用，获取文件状态
  fstat — 系统调用，通过 fd 获取文件状态
  struct stat — 文件状态结构体
Predefined Macros/Crates:
  syscall 模块 (rusl 内部 syscall 封装)
  ERANGE — 缓冲区大小不足错误码
  ENODEV — 设备不匹配错误码

[GUARANTEE]
Exported Interface:
  extern "C" fn ttyname_r(fd: core::ffi::c_int, name: *mut core::ffi::c_char, size: usize) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 ttyname_r 符号
Internal Interface:
  pub(crate) fn tty_name_for_fd(fd: RawFd) -> Result<String, Errno>;
                                 // 安全 Rust 包装
