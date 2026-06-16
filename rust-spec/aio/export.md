# aio 模块 — 对外导出 API 汇总

本文件记录 `rusl-aio` crate 的所有对外导出接口（由 `<aio.h>` 声明），以及对应的 Rust `extern "C"` ABI 设计。所有符号均须保持与 C ABI 完全兼容。

---

## 公共数据类型（用户直接可见）

```rust
use core::ffi::{c_int, c_uint, c_void, c_char};

/// POSIX 异步 I/O 控制块
///
/// 在 Rust 中以 `#[repr(C)]` 标记保证与 C 的 `struct aiocb` 内存布局一致。
/// 字段定义与 `<aio.h>` 中的 C 结构体完全对应。
#[repr(C)]
pub struct aiocb {
    pub aio_fildes: c_int,           // 文件描述符
    pub aio_lio_opcode: c_int,       // lio_listio 操作码 (LIO_READ/LIO_WRITE/LIO_NOP)
    pub aio_reqprio: c_int,          // 请求优先级
    pub aio_buf: *mut c_void,        // I/O 缓冲区（volatile 修饰，在 Rust 中以 UnsafeCell 或 *mut 表示）
    pub aio_nbytes: usize,           // 待传输字节数
    pub aio_sigevent: sigevent,      // 完成通知结构
    // 内部字段 (musl 实现细节，用户不应直接访问):
    pub __pad: [c_int; 1],           // 填充
    pub __next: *mut aiocb,          // 链表指针
    pub __prev: *mut aiocb,          // 链表指针
    pub __td: *mut c_void,           // 所属线程指针 (pthread_t)
    pub __ss: isize,                 // 实际 I/O 字节数 (ssize_t)
    pub __err: c_int,                // 完成状态码，低 31 位为错误码，bit 31 为"有等待者"标志
}
```

注：`aiocb` 内部字段的精确偏移和布局由 POSIX 标准和 musl 实现定义。`aio_buf` 在 C 中声明为 `volatile void *`；在 Rust 中，通过 `*mut c_void` 访问时在 `unsafe` 块内使用 `read_volatile`/`write_volatile` 保持 volatile 语义。

---

## 公共常量（用户直接可见）

| 符号 | Rust 定义 | 值 | 说明 |
|------|----------|-----|------|
| `AIO_CANCELED` | `pub const AIO_CANCELED: c_int = 0;` | `0` | AIO 操作已被取消 |
| `AIO_NOTCANCELED` | `pub const AIO_NOTCANCELED: c_int = 1;` | `1` | AIO 操作未被取消（至少一个已完成） |
| `AIO_ALLDONE` | `pub const AIO_ALLDONE: c_int = 2;` | `2` | 所有 AIO 操作已完成（无需取消） |
| `LIO_READ` | `pub const LIO_READ: c_int = 0;` | `0` | 异步读操作码 |
| `LIO_WRITE` | `pub const LIO_WRITE: c_int = 1;` | `1` | 异步写操作码 |
| `LIO_NOP` | `pub const LIO_NOP: c_int = 2;` | `2` | 空操作（无 I/O） |
| `LIO_WAIT` | `pub const LIO_WAIT: c_int = 0;` | `0` | lio_listio 阻塞等待所有操作完成 |
| `LIO_NOWAIT` | `pub const LIO_NOWAIT: c_int = 1;` | `1` | lio_listio 异步启动，立即返回 |

```rust
pub const AIO_CANCELED: core::ffi::c_int = 0;
pub const AIO_NOTCANCELED: core::ffi::c_int = 1;
pub const AIO_ALLDONE: core::ffi::c_int = 2;
pub const LIO_READ: core::ffi::c_int = 0;
pub const LIO_WRITE: core::ffi::c_int = 1;
pub const LIO_NOP: core::ffi::c_int = 2;
pub const LIO_WAIT: core::ffi::c_int = 0;
pub const LIO_NOWAIT: core::ffi::c_int = 1;
```

---

## 1. 异步 I/O 操作控制

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `aio_read` | `pub extern "C" fn aio_read(cb: *mut aiocb) -> c_int;` | 提交异步读请求 |
| `aio_write` | `pub extern "C" fn aio_write(cb: *mut aiocb) -> c_int;` | 提交异步写请求 |
| `aio_fsync` | `pub extern "C" fn aio_fsync(op: c_int, cb: *mut aiocb) -> c_int;` | 提交异步 fsync 请求（op: O_SYNC/O_DSYNC） |
| `aio_error` | `pub extern "C" fn aio_error(cb: *const aiocb) -> c_int;` | 查询 AIO 操作状态，返回错误码或 EINPROGRESS |
| `aio_return` | `pub extern "C" fn aio_return(cb: *mut aiocb) -> isize;` | 获取已完成 AIO 操作的返回值（ssize_t），每操作仅可调用一次 |
| `aio_cancel` | `pub extern "C" fn aio_cancel(fd: c_int, cb: *mut aiocb) -> c_int;` | 取消文件描述符上的异步 I/O 操作 |
| `aio_suspend` | `pub extern "C" fn aio_suspend(cbs: *const *const aiocb, cnt: c_int, ts: *const timespec) -> c_int;` | 挂起线程直到至少一个 AIO 操作完成、超时或收到信号 |
| `lio_listio` | `pub extern "C" fn lio_listio(mode: c_int, cbs: *const *mut aiocb, cnt: c_int, sev: *mut sigevent) -> c_int;` | 批量提交异步 I/O 请求列表 |

所有上述函数均为 POSIX 标准接口，声明于 `<aio.h>`，用户可直接调用。

---

## 2. musl `__` 前缀内部符号（rusl 必须同时导出）

musl 中 `__xxx` 是主实现函数或模块间共享变量，`rusl` 必须同时提供这些内部导出符号。这些符号在 Rust 侧以 `pub extern "C"` 导出，但标注为内部实现细节（用户不应直接使用）。

| 内部符号 | Rust extern "C" 签名 | 类型 | 说明 |
|---------|---------------------|------|------|
| `__aio_fut` | `pub static __aio_fut: AtomicI32;` (内部以 `AtomicI32` 实现，对外暴露为 `extern "C"` 可寻址的 `c_int` 变量) | `volatile int` | 多 aiocb 挂起时的全局 futex 等待字。`cleanup()` 中通过 `a_swap` 置零并唤醒等待者。 |
| `__aio_close` | `pub extern "C" fn __aio_close(fd: c_int) -> c_int;` | `int (*)(int)` | 由 `close()` 调用以取消关联文件描述符上的未完成 AIO 操作。 |
| `__aio_atfork` | `pub extern "C" fn __aio_atfork(who: c_int);` | `void (*)(int)` | 由 `fork()` 调用以处理 fork 后的 AIO 状态清理。`who` 参数指示当前处于 fork 的哪个阶段。 |

---

## 排除说明

以下类别的符号不需要在 Rust 外部 ABI 中导出：

- 所有 `static` 内部函数（已重构为 Rust `pub(crate)` 或更小可见性的私有函数）
- 内部静态变量（已重构为 Rust `static`，如模块级别的链表头、互斥锁等）
- `__aio_fut` 内部使用 Rust `AtomicI32` 实现，对外 ABI 须保持为可寻址的全局 `c_int` 变量
- 架构特定实现（作为 `cfg(target_arch = "...")` 的条件编译模块）

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  aiocb — rusl-aio 内部定义的 repr(C) 结构体
  timespec — rusl-time crate 提供的 repr(C) 时间结构
  sigevent — rusl-signal crate 提供的 repr(C) 通知结构
  rusl-syscall 提供的系统调用封装接口
Predefined Macros/Crates:
  core::ffi — Rust 核心库的 FFI 类型 (c_int, c_uint, c_char, c_void, c_ulong)
  core::sync::atomic — Rust 核心库的原子类型

[GUARANTEE]
Exported Interface:
  上述所有符号均须以 pub extern "C" 导出，保证与 C ABI 完全兼容
  musl __ 前缀内部符号必须同时导出（作为独立符号，非弱别名）
  __aio_fut 在 Rust 内部以 AtomicI32 实现，但对外必须暴露为与 C 一致的可链接符号（c_int 大小的全局变量地址）
