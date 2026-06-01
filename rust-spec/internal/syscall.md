# syscall 规约 (Rust)

> **来源文件**: `musl/src/internal/syscall.h`
> **目标模块**: `rusl/src/internal/syscall.rs`
> **复杂度层级**: Level 3 — 高度优化设计（宏调度系统 + 平台兼容层）

---

## 概述

`syscall` 模块是 rusl 系统调用层的核心模块。它定义了统一的系统调用调度机制，将各架构的原始 syscall 指令包装为类型安全的 Rust 函数和宏，并提供以下关键服务：
1. **参数类型统一** (`SyscallArg`)：所有 syscall 参数统一转为 `isize` 类型，避免 32/64 位架构差异
2. **可变参数调度** (`syscall!` 宏)：根据参数个数自动选择 `syscall1` ~ `syscall7`
3. **返回值标准化** (`syscall_ret`)：将内核返回的负数 errno 转为 `-1` 并设置 errno
4. **取消点支持** (`syscall_cp` 系列)：在系统调用前后检查线程取消标志
5. **平台兼容修复** (`#[cfg(...)]`)：处理 32/64 位 ABI 差异、time64 过渡、socketcall 多路复用

**不变量 (Invariants)**：
- **I1**: 所有经 `syscall_ret` 处理的返回值保证：若调用成功返回非负值，若失败则 errno 被设置为正值且函数返回 `-1`。
- **I2**: 经 `syscall_cp` 系列的调用在进入内核前检查取消点；若线程已被取消，系统调用**绝不执行**。
- **I3**: 所有 syscall 参数均转为 `isize`，保证寄存器宽度一致性。

---

## [RELY]

```
Predefined Types/Functions:
  type c_int = i32;
  type c_long = isize;
  type c_ulong = usize;
  use crate::internal::syscall_arch::{syscall0, syscall1, syscall2, syscall3, syscall4, syscall5, syscall6};
                                  // 依赖: 架构相关内联汇编实现
  fn syscall_ret(r: usize) -> isize;
                                  // 依赖: 系统调用返回值 errno 转换
  fn syscall_cp_raw(sysno: isize, a: isize, b: isize, c: isize, d: isize, e: isize, f: isize) -> isize;
                                  // 依赖: 带取消点的系统调用
  fn emulate_wait4(a: isize, b: isize, c: isize, d: isize) -> isize;
                                  // 依赖: wait4 模拟（条件编译时可见）
  fn procfdname(buf: &mut [u8; 15 + 3 * core::mem::size_of::<c_int>()], fd: c_uint);
                                  // 依赖: 文件描述符路径生成
  fn vdsosym(vername: *const c_char, name: *const c_char) -> *mut c_void;
                                  // 依赖: vDSO 符号查找
```

## [GUARANTEE]

```
Exported Interface (保持 ABI 兼容):
  // 返回值处理器 — 被所有系统调用宏内部使用
  fn __syscall_ret(r: c_ulong) -> c_long;          // [Visibility]: Internal
  fn __vdsosym(vername: *const c_char, name: *const c_char) -> *mut c_void;  // [Visibility]: Internal
  fn __procfdname(buf: *mut c_char, fd: c_uint);   // [Visibility]: Internal

Internal Interface（宏展开体系，不对外导出）:
  // 类型
  type SyscallArg = isize;  // syscall_arg_t → Rust 侧统一为 isize

  // 底层宏 — 展开后调用 syscall_arch 模块
  macro_rules! syscall { ... }         // 执行原始系统调用，返回未处理的 isize 值
  macro_rules! syscall_ret { ... }     // 执行系统调用并自动进行 errno 转换（即原 C 的 syscall(...) 宏）
  macro_rules! syscall_cp { ... }      // 带取消点的原始系统调用
  macro_rules! syscall_cp_ret { ... }  // 带取消点并自动 errno 转换

  // Socket 统一调度
  fn alt_socketcall(sys: c_int, sock: c_int, cp: c_int, a..f: SyscallArg) -> c_long;

  // 平台兼容宏
  macro_rules! sys_open { ... }   // open/openat 统一
  macro_rules! sys_pause { ... }  // pause/ppoll 统一

  // 常量
  const SYSCALL_RLIM_INFINITY: u64 = !0u64;
  const SYSCALL_MMAP2_UNIT: u64 = 4096;
  const SC_SOCKET: c_int = 1;
  ... (socket 子调用号常量列表)
```

---

## 类型定义

### `SyscallArg`

```rust
/// 系统调用参数的统一类型。64位架构为 64 位，32 位架构为 32 位。
pub(crate) type SyscallArg = isize;
```

`[Visibility]: Internal`

---

## 核心函数声明

### `__syscall_ret`

```rust
/// 将 Linux 内核系统调用的原始返回值转换为符合 C 库约定的返回值
///
/// # Safety
/// 参数 `r` 必须是某次 Linux 系统调用的原始返回值。
#[no_mangle]
pub unsafe extern "C" fn __syscall_ret(r: c_ulong) -> c_long;
```

`[Visibility]: Internal — musl 内部系统调用返回值处理器`

**意图 (Intent)**：将 Linux 内核系统调用的原始返回值（负数 = -errno 表示错误，非负 = 成功）转换为符合 C 库约定的返回值。

**系统算法**：
```rust
pub unsafe extern "C" fn __syscall_ret(r: c_ulong) -> c_long {
    if r > (-4096isize) as c_ulong {
        // Case: 错误 — r 在 [0xFFFFFFFFFFFFF000, 0xFFFFFFFFFFFFFFFF] 范围
        // errno = -(r as isize)
        -1
    } else {
        r as c_long
    }
}
```

---

### `__syscall_cp`

```rust
/// 执行系统调用前检查线程取消标志
///
/// # Safety
/// 调用线程必须处于可取消状态。第 7 个参数为有效的 Linux syscall 号。
#[no_mangle]
pub unsafe extern "C" fn __syscall_cp(
    a: SyscallArg, b: SyscallArg, c: SyscallArg,
    d: SyscallArg, e: SyscallArg, f: SyscallArg,
    sysno: SyscallArg,
) -> c_long;
```

`[Visibility]: Internal`

**意图 (Intent)**：执行系统调用前检查线程取消标志，若线程已标记取消则立即执行取消操作而不进入内核。

**前置条件**：
- 调用线程必须处于可取消状态。
- 第 7 个参数为有效的 Linux syscall 号。

---

### `__alt_socketcall` — Socket 调用统一调度器

```rust
/// 在新架构上直接调用 socket syscall；在旧架构上退化为 socketcall() 多路复用
pub(crate) fn alt_socketcall(
    sys: c_int, sock: c_int, cp: c_int,
    a: SyscallArg, b: SyscallArg, c: SyscallArg,
    d: SyscallArg, e: SyscallArg, f: SyscallArg,
) -> c_long;
```

`[Visibility]: Internal — 内部函数，不导出`

**系统算法**：
```
1. 若 cp != 0 → r = __syscall_cp(sys, a, b, c, d, e, f)
   否则       → r = __syscall6(sys, a, b, c, d, e, f)
2. 若 r != -ENOSYS → return r
3. 若 cfg!(SYS_socketcall 定义):
     若 cp != 0 → r = __syscall_cp(SYS_socketcall, sock, 0..)
     否则       → r = __syscall6(SYS_socketcall, sock, 0..)
4. return r
```

---

### `__procfdname`

```rust
/// 将文件描述符号转换为对应的 /proc/self/fd/%u 路径字符串
#[no_mangle]
pub unsafe extern "C" fn __procfdname(buf: *mut c_char, fd: c_uint);
```

`[Visibility]: Internal`

---

### `__vdsosym`

```rust
/// 从 Linux vDSO 中按名称和版本查找函数指针
#[no_mangle]
pub unsafe extern "C" fn __vdsosym(vername: *const c_char, name: *const c_char) -> *mut c_void;
```

`[Visibility]: Internal`

**意图 (Intent)**：从 Linux vDSO 中查找符号地址，用于快速获取 `clock_gettime` 等高频系统调用的用户态实现。

**前置条件**：
- `vername` 非空，为有效的 ELF 版本字符串（如 `"LINUX_2.6"`）。
- `name` 非空，为有效的 ELF 符号名（如 `"__vdso_clock_gettime"`）。

**后置条件**：
- 成功: 返回 vDSO 中对应符号的地址。
- 失败: 返回 `null_mut()`。

---

## 系统调用调度宏体系

以下宏构成统一的可变参数系统调用调度系统，均为 **Internal** 可见性。

### 参数数量计数宏

```rust
/// 编译期计算参数个数
macro_rules! syscall_nargs {
    ($($args:expr),*) => { ... };  // 展开为参数个数
}
```

### 参数拼接调度宏

```rust
/// 核心调度器：syscall!{SYS_read, fd, buf, len} → syscall3(SYS_read, fd, buf, len)
macro_rules! syscall {
    ($sysno:expr $(, $arg:expr)*) => { ... };
}

/// 原始系统调用 + 自动 errno 转换（对应 C 的 syscall(...) 宏）
macro_rules! syscall_ret {
    ($sysno:expr $(, $arg:expr)*) => {{
        let r = syscall!($sysno $(, $arg)*);
        __syscall_ret(r)
    }};
}

/// 带取消点的原始系统调用
macro_rules! syscall_cp {
    ($sysno:expr $(, $arg:expr)*) => { ... };
}

/// 带取消点 + 自动 errno 转换（对应 C 的 syscall_cp(...) 宏）
macro_rules! syscall_cp_ret {
    ($sysno:expr $(, $arg:expr)*) => {{
        let r = syscall_cp!($sysno $(, $arg)*);
        __syscall_ret(r)
    }};
}
```

**注意**：Rust 的 `macro_rules!` 不支持 C 预处理器风格的 `##` 符号拼接。替代方案：
1. 使用 `macro_rules!` 的 `$(...)*` 重复计数模式实现参数个数检测
2. 通过 match 参数个数显式分派到对应的 `syscallN` 函数

---

## Socket 调用子系统

### Socket 子调用号常量

```rust
pub(crate) const SC_SOCKET: c_int      = 1;
pub(crate) const SC_BIND: c_int        = 2;
pub(crate) const SC_CONNECT: c_int     = 3;
pub(crate) const SC_LISTEN: c_int      = 4;
pub(crate) const SC_ACCEPT: c_int      = 5;
pub(crate) const SC_GETSOCKNAME: c_int = 6;
pub(crate) const SC_GETPEERNAME: c_int = 7;
pub(crate) const SC_SOCKETPAIR: c_int  = 8;
pub(crate) const SC_SEND: c_int        = 9;
pub(crate) const SC_RECV: c_int        = 10;
pub(crate) const SC_SENDTO: c_int      = 11;
pub(crate) const SC_RECVFROM: c_int    = 12;
pub(crate) const SC_SHUTDOWN: c_int    = 13;
pub(crate) const SC_SETSOCKOPT: c_int  = 14;
pub(crate) const SC_GETSOCKOPT: c_int  = 15;
pub(crate) const SC_SENDMSG: c_int     = 16;
pub(crate) const SC_RECVMSG: c_int     = 17;
pub(crate) const SC_ACCEPT4: c_int     = 18;
pub(crate) const SC_RECVMMSG: c_int    = 19;
pub(crate) const SC_SENDMMSG: c_int    = 20;
```

`[Visibility]: Internal`

---

## 文件操作统一接口

### open/openat 统一

```rust
#[cfg(not(target_arch = "..."))]  // 根据架构选择
macro_rules! sys_open { ... }  // 使用 SYS_open
#[cfg(target_arch = "...")]    // 新架构
macro_rules! sys_open { ... }  // 使用 openat(AT_FDCWD, ...)
```

### pause/ppoll 统一

```rust
#[cfg(not(target_arch = "..."))]
pub(crate) fn sys_pause() -> c_long { ... }  // 使用 SYS_pause
#[cfg(target_arch = "...")]
pub(crate) fn sys_pause() -> c_long { ... }  // 退化为 ppoll
```

### wait4 可选模拟

```rust
#[cfg(not(target_arch = "..."))]
pub(crate) fn sys_wait4(...) -> c_long { ... }  // 使用 SYS_wait4
#[cfg(target_arch = "...")]
pub(crate) fn sys_wait4(...) -> c_long { ... }  // 退化为 emulate_wait4
```

---

## 平台兼容性修复

使用 Rust 的条件编译替代 C 的 `#ifdef SYS_*`：

```rust
// 大文件支持修复
#[cfg(target_arch = "...")]
const SYS_stat: c_int = SYS_stat64;
#[cfg(target_arch = "...")]
const SYS_fstat: c_int = SYS_fstat64;
...

// Time64 修复
#[cfg(target_arch = "...")]
const SYS_clock_gettime: c_int = SYS_clock_gettime64;
...

// Socket 超时选项常量
const SO_RCVTIMEO_OLD: c_int = 20;
const SO_SNDTIMEO_OLD: c_int = 21;
const SO_TIMESTAMP_OLD: c_int = 29;
const SO_TIMESTAMPNS_OLD: c_int = 35;
const SO_TIMESTAMPING_OLD: c_int = 37;

// accept4 降级
#[cfg(not(defined(SYS_accept)))]
const SYS_accept: c_int = SYS_accept4;
```

---

## 跨文件依赖

| 依赖符号 | 来源 | 处理方式 |
|---------|------|---------|
| `syscall0` ~ `syscall6` | `syscall_arch` 模块（架构相关） | 外部依赖，由具体架构的 ABI 约定实现 |
| `__syscall_ret()` | `syscall_ret` 模块 | 跨文件实现 |
| `__syscall_cp()` | `thread` 相关模块 | 跨文件实现 |
| `emulate_wait4()` | `process` 相关模块 | 跨文件实现，条件编译时可见 |
| `__procfdname()` | `procfdname` 模块 | 跨文件实现 |
| `__vdsosym()` | `vdso` 模块 | 跨文件实现 |
| `SYS_*` 常量 | Linux 内核定义 | 按架构条件编译定义 |

---

## 实现指南 (rusl/Rust)

- `syscall_arg_t` → `pub(crate) type SyscallArg = isize;`
- `__syscall_ret()` → ```fn syscall_ret(r: usize) -> isize```: `if r > -4096isize as usize { set_errno(-(r as c_int)); -1 } else { r as isize }`
- `__syscall_cp()` → 在调用 syscall 前通过原子变量检查取消标志
- 可变参数调度 → 使用 Rust `macro_rules!` 实现计数 + 分派机制
- 平台兼容 `#ifdef` → 使用 Rust 的 `#[cfg(target_arch = "...")]` 以及 `cfg!()` 宏
- `__alt_socketcall()` → 普通函数，利用 Rust 编译器的内联优化
- vDSO 查找 → 解析 `auxv` 获取 vDSO 基址，手动 ELF 符号查找
- `errno` → 使用线程局部的 `errno` 实现