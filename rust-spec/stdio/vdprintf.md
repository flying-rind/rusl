# vdprintf — Rust 接口归约

## 复杂度分级: Level 2

> musl libc `va_list` 版文件描述符格式化输出函数。通过构造最小伪 `FILE` 对象并委托 `vfprintf` 实现。

---

## 原始 C 接口
```c
int vdprintf(int fd, const char *restrict fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出（POSIX 扩展）

---

## Rust 外部 ABI 接口

```rust
// C ABI 兼容: va_list 通过 core::ffi::VaList 传递
extern "C" fn vdprintf(
    fd: core::ffi::c_int,
    fmt: *const core::ffi::c_char,
    ap: core::ffi::VaList,
) -> core::ffi::c_int;
```

---

## Rust 安全接口设计

```rust
// Rust 原生的 vdprintf 等价物——直接输出到文件描述符
pub fn rust_vdprintf(fd: RawFd, fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
```

内部实现通过构造一个最小 `RustFile` 适配器对象，其 `write` 方法直接执行系统调用 `write(fd, ...)`，无缓冲模式。然后将此适配器传入 `rust_vfprintf` 引擎。

```rust
// 内部使用的文件描述符写入适配器（不对外暴露）
pub(crate) struct RawFdWriter {
    fd: RawFd,
}

impl RawFdWriter {
    /// 直接系统调用写入，无缓冲
    fn write(&mut self, buf: &[u8]) -> Result<usize, IoError> {
        // syscall write(self.fd, buf.as_ptr(), buf.len())
    }
}
```

---

## 意图

将格式化字符串写入文件描述符 `fd`。通过构造一个最小伪 `FILE` 对象绕过 `FILE*` 流机制直接写入文件描述符。

## 前置条件

- `fd` 为有效的文件描述符
- `fmt != NULL`，指向有效的格式化字符串
- `ap` 已由 `va_start` 正确初始化

## 后置条件

- Case 1 成功：返回写入 `fd` 的字符总数
- Case 2 输出错误：返回 `-1`
- Case 3 格式错误：返回 `-1`，`errno = EINVAL`
- Case 4 溢出：返回 `-1`，`errno = EOVERFLOW`

## 不变量

- 伪 `FILE`/适配器对象仅在栈上存在，函数返回后销毁
- 不使用缓冲，每次写入直接通过系统调用 `write` 完成
- 无锁模式（伪流不会被多个线程共享）

## 算法

原 C 实现：
```
vdprintf(fd, fmt, ap):
  1. 在栈上构造 FILE 对象：
     .fd = fd
     .lbf = EOF (无行缓冲)
     .write = __stdio_write (直接系统调用写入)
     .buf = (void *)fmt (空操作指针，无实际缓冲)
     .buf_size = 0 (无缓冲模式)
     .lock = -1 (禁用锁定)
  2. return vfprintf(&f, fmt, ap)
```

Rust 实现路径：

### 路径 A：C ABI 兼容（extern "C"）
1. 在栈上构造 `FILE` 对象（AArch64/RISC-V 对齐要求 ~80 字节）
2. 调用 `vfprintf(&f, fmt, ap)`（C ABI 路径）

### 路径 B：纯 Rust 实现（内部使用）
1. 构造 `RawFdWriter { fd }` 适配器
2. 解析 `FormatArg` 列表，逐一格式化写入
3. 每次写入直接调用 `write` 系统调用
4. 返回总写入字节数

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  int vfprintf(FILE *f, const char *fmt, va_list ap);
                                   // 依赖1: C ABI vfprintf 实现（核心引擎）
  size_t __stdio_write(FILE *f, const unsigned char *buf, size_t len);
                                   // 依赖2: 直接文件描述符写入
  struct FILE { int fd; int lbf; ... }
                                   // 依赖3: FILE 结构体定义（来自 stdio_impl.h）
  core::ffi::VaList                  // 依赖4: Rust 内置 va_list 类型
  pub(crate) fn rust_vfprintf(f: &mut RustFile, fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                   // 依赖5: Rust 内部格式化引擎
  pub(crate) enum FormatArg { ... }
                                   // 依赖6: 格式化参数类型（来自 vfprintf 模块）

[GUARANTEE]
Exported Interface:
  extern "C" fn vdprintf(
      fd: core::ffi::c_int,
      fmt: *const core::ffi::c_char,
      ap: core::ffi::VaList,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 vdprintf 符号
Internal Interface:
  pub fn rust_vdprintf(fd: RawFd, fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                 // 安全的 Rust 原生格式化接口
  pub(crate) struct RawFdWriter { fd: RawFd }
                                 // 文件描述符写入适配器（模块内部）
