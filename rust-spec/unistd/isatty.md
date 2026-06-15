# isatty — Rust 接口归约

## 原始 C 接口
```c
int isatty(int fd);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn isatty(fd: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图

测试文件描述符 `fd` 是否关联到一个终端设备。通过尝试对 `fd` 执行 `TIOCGWINSZ`（获取终端窗口大小）ioctl 系统调用来判断：若调用成功（返回 0），`fd` 即为终端；若调用失败（返回 -1），`fd` 不是终端。利用 "+1" 技巧将错误状态（0/-1）转换为布尔值（1/0）。

## 前置条件

- `fd`: 有效的已打开文件描述符（无效 fd 会被判定为非终端，返回 0）

## 后置条件

- **Case 1 `fd` 关联终端设备**
  - 返回 1（true）
  - `errno` 不变

- **Case 2 `fd` 不关联终端设备**
  - 返回 0（false）
  - `errno` 可能被设置（如 `EBADF`、`ENOTTY`），但函数不依赖 errno 做判断

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现调用 `syscall(SYS_ioctl, fd, TIOCGWINSZ, &wsz) + 1`。Rust 中：

```rust
extern "C" fn isatty(fd: core::ffi::c_int) -> core::ffi::c_int {
    let mut wsz = Winsize::default();
    // ioctl 返回 0(成功) → +1 = 1(是终端)，-1(失败) → +1 = 0(不是终端)
    unsafe { syscall_with_ret(SYS_ioctl, fd, TIOCGWINSZ, &mut wsz) } + 1
}
```

即：
1. 声明 `Winsize` 作为 ioctl 输出缓冲区（实际不需要读取结果）
2. 调用 `syscall(SYS_ioctl, fd, TIOCGWINSZ, &wsz)` 执行内核 ioctl 系统调用
3. 返回值 +1：0 变为 1（是终端），-1 变为 0（不是终端）

---

## Rust 安全包装（模块内部）

```rust
/// 安全包装，返回 bool
pub(crate) fn is_terminal(fd: RawFd) -> bool {
    let mut wsz = Winsize::default();
    // 直接调用 ioctl
    match unsafe { sys_ioctl(fd, TIOCGWINSZ, &mut wsz) } {
        0 => true,
        _ => false,
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_ioctl syscall — Linux 内核系统调用编号 (x86_64: 16, aarch64: 29)
  TIOCGWINSZ — 获取终端窗口大小的 ioctl 请求码 (0x5413)
  struct winsize — 终端窗口大小结构体，定义在 <sys/ioctl.h>
Predefined Macros/Crates:
  syscall 模块 (rusl 内部 syscall 封装)

[GUARANTEE]
Exported Interface:
  extern "C" fn isatty(fd: core::ffi::c_int) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 isatty 符号
Internal Interface:
  pub(crate) fn is_terminal(fd: RawFd) -> bool;
                                 // 安全 Rust 包装，返回 bool
