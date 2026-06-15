# ttyname — Rust 接口归约

## 原始 C 接口
```c
char *ttyname(int fd);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn ttyname(fd: core::ffi::c_int) -> *mut core::ffi::c_char;
```

---

## 意图

获取文件描述符 `fd` 所关联的终端设备路径名。将线程安全版本 `ttyname_r` 的结果存入内部静态缓冲区，提供简化的非线程安全接口。

## 前置条件

- `fd`: 有效的已打开文件描述符

## 后置条件

- **Case 1 `fd` 关联终端设备**
  - 返回指向内部静态缓冲区 `buf` 的指针
  - `buf` 内容为终端设备路径名（如 `/dev/pts/0`）
  - 后续调用 `ttyname` 会覆盖 `buf` 内容

- **Case 2 `fd` 不关联终端设备或发生错误**
  - 返回 `NULL`
  - `errno` 设置为 `ttyname_r` 返回的错误码（如 `ENOTTY`、`EBADF`、`ENODEV`、`ERANGE`）

## 不变量

- 内部静态缓冲区 `BUF` 大小为 `TTY_NAME_MAX`（32 字节）

## 算法

原 C 实现使用静态缓冲区调用 `ttyname_r`，根据返回值设置 errno 并返回指针。Rust 中：

```rust
// 内部静态缓冲区 — 使用 Mutex<[u8; TTY_NAME_MAX]> 保护线程安全
static TTY_NAME_BUF: SyncUnsafeCell<[u8; TTY_NAME_MAX]> = SyncUnsafeCell::new([0u8; TTY_NAME_MAX]);

extern "C" fn ttyname(fd: core::ffi::c_int) -> *mut core::ffi::c_char {
    let buf_ptr = unsafe { (*TTY_NAME_BUF.get()).as_mut_ptr() };
    let ret = unsafe { ttyname_r(fd, buf_ptr as *mut i8, TTY_NAME_MAX) };
    if ret != 0 {
        unsafe { set_errno(ret) };
        core::ptr::null_mut()
    } else {
        buf_ptr
    }
}
```

注意：Rust 中 static mut 不够安全，若需要线程安全性，可使用 `Mutex` 包装（但这会改变 musl 的非线程安全语义）。按照 musl 原始设计，`ttyname` 本身不保证线程安全，因此此处直接使用内部可变静态变量即可。

---

## Rust 安全包装（模块内部）

```rust
/// 安全包装 — 对 API 使用者提供线程安全版本
/// 直接委托给 ttyname_r 的安全包装
pub(crate) fn terminal_name(fd: RawFd) -> Result<String, Errno> {
    tty_name_for_fd(fd)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  ttyname_r(fd, buf, size) — 线程安全版终端名获取
  TTY_NAME_MAX — 终端名称最大长度宏，值为 32
Predefined Macros/Crates:
  errno 模块 (rusl 内部 errno 管理)

[GUARANTEE]
Exported Interface:
  extern "C" fn ttyname(fd: core::ffi::c_int) -> *mut core::ffi::c_char;
                                 // 本模块保证对外提供与 C ABI 兼容的 ttyname 符号
Internal Interface:
  pub(crate) fn terminal_name(fd: RawFd) -> Result<String, Errno>;
                                 // 安全 Rust 包装（委托给 ttyname_r 的包装）
