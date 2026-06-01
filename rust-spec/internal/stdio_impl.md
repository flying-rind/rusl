# stdio_impl 规约 (Rust)

> **来源文件**: `musl/src/internal/stdio_impl.h`
> **目标模块**: `rusl/src/internal/stdio_impl.rs`
> **复杂度层级**: Level 3 — 高度优化设计（完整 stdio 抽象 + 文件描述符管理层 + 线程安全锁）

---

## 概述

`stdio_impl` 模块是 rusl 标准 I/O 库的内部核心。它定义了 `struct _IO_FILE`（即 `FILE` 的真身）、所有内部缓冲 I/O 操作的函数接口，以及线程安全的文件锁管理机制。

**不变量 (Invariants)**：
- **I1**: 任何对 `FILE` 中 `rpos/rend/wpos/wend` 的修改必须在持有 `lock` 的状态下进行（多线程安全），或调用者保证单线程访问。
- **I2**: `buf` 和 `buf_size` 要么均为 0（无缓冲），要么描述一个有效缓冲区。`wbase` 指向写缓冲起始位置。
- **I3**: `prev/next` 构成全局打开文件双向链表，由打开文件链表的锁保护。
- **I4**: `flags` 中的 `F_ERR` 和 `F_EOF` 一旦设置，不会被主动清除（除非显式调用 `clearerr`）。
- **I5**: `pipe_pid` 若为非零值，表示该 FILE 通过 `popen()` 创建，关闭时需回收子进程。

---

## [RELY]

```
Predefined Structures/Functions:
  // 系统调用层
  fn sys_read(fd: c_int, buf: *mut c_void, len: size_t) -> isize;
  fn sys_write(fd: c_int, buf: *const c_void, len: size_t) -> isize;
  fn sys_close(fd: c_int) -> isize;
  fn sys_lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t;
  // futex 原语
  fn futex_wait(addr: *const AtomicI32, val: i32) -> c_int;
  fn futex_wake(addr: *const AtomicI32, cnt: c_int) -> c_int;
  // 原子操作
  use core::sync::atomic::{AtomicI32, Ordering};
  // 类型别名
  type c_int = i32;
  type c_void = core::ffi::c_void;
  type size_t = usize;
  type off_t = i64;
  // bitflags
  use bitflags::bitflags;
  // 线程相关（跨模块）
  struct __pthread { ... };              // 依赖: 线程结构体
```

## [GUARANTEE]

```
Exported Interface (保持 ABI 兼容):
  // struct _IO_FILE — #[repr(C)] 布局与 C 侧完全兼容
  // FILE 类型别名: pub type FILE = _IO_FILE;
  
  // 半公共接口（protected 可见性）
  fn __overflow(f: *mut FILE, c: c_int) -> c_int;
  fn __uflow(f: *mut FILE) -> c_int;
  
  // 内部锁操作（被 FLOCK/FUNLOCK 宏调用，或在 Rust 侧被 RAII 守卫调用）
  fn __lockfile(f: *mut FILE) -> c_int;
  fn __unlockfile(f: *mut FILE);
  
  // 生命周期
  fn __stdio_exit();
  fn __stdio_exit_needed();

Internal Interface（模块内部使用，不对外导出）:
  fn __stdio_read(f: *mut FILE, buf: *mut u8, len: size_t) -> size_t;
  fn __stdio_write(f: *mut FILE, buf: *const u8, len: size_t) -> size_t;
  fn __stdout_write(f: *mut FILE, buf: *const u8, len: size_t) -> size_t;
  fn __stdio_seek(f: *mut FILE, off: off_t, whence: c_int) -> off_t;
  fn __stdio_close(f: *mut FILE) -> c_int;
  fn __toread(f: *mut FILE) -> c_int;
  fn __towrite(f: *mut FILE) -> c_int;
  fn __fseeko(f: *mut FILE, off: off_t, whence: c_int) -> c_int;
  fn __fseeko_unlocked(f: *mut FILE, off: off_t, whence: c_int) -> c_int;
  fn __ftello(f: *mut FILE) -> off_t;
  fn __ftello_unlocked(f: *mut FILE) -> off_t;
  fn __fwritex(s: *const u8, l: size_t, f: *mut FILE) -> size_t;
  fn __putc_unlocked(c: c_int, f: *mut FILE) -> c_int;
  fn __fdopen(fd: c_int, mode: *const c_char) -> *mut FILE;
  fn __fmodeflags(mode: *const c_char) -> c_int;
  fn __ofl_add(f: *mut FILE) -> *mut FILE;
  fn __ofl_lock() -> *mut *mut FILE;
  fn __ofl_unlock();
  fn __register_locked_file(f: *mut FILE, td: *mut __pthread);
  fn __unlist_locked_file(f: *mut FILE);
  fn __do_orphaned_stdio_locks();
  fn __getopt_msg(a: *const c_char, b: *const c_char, c: *const c_char, l: size_t);
  fn __fopen_rb_ca(filename: *const c_char, f: *mut FILE, buf: *mut u8, sz: size_t) -> *mut FILE;
  fn __fclose_ca(f: *mut FILE) -> c_int;
  
  // 全局变量
  static __stdin_used: *mut FILE;   // 内部使用的 stdin 指针
  static __stdout_used: *mut FILE;  // 内部使用的 stdout 指针
  static __stderr_used: *mut FILE;  // 内部使用的 stderr 指针
```

---

## 常量定义 (Rust 风格)

```rust
/// ungetc 推回缓冲区的大小（字节数）
const UNGET: usize = 8;
```

### 文件状态标志 (flags)

```rust
bitflags! {
    #[repr(C)]
    pub struct FileFlags: c_uint {
        /// 永久分配（不由 fclose 释放 FILE 结构自身）
        const F_PERM = 0x01;
        /// 不可读（流已关闭读取方向）
        const F_NORD = 0x04;
        /// 不可写（流已关闭写入方向）
        const F_NOWR = 0x08;
        /// 已遇到文件末尾
        const F_EOF  = 0x10;
        /// 发生 I/O 错误
        const F_ERR  = 0x20;
        /// 行缓冲（Line-buffered）
        const F_SVB  = 0x40;
        /// 追加模式 (O_APPEND)
        const F_APP  = 0x80;
    }
}
```

### 锁常量

```rust
/// FILE lock 字段中标记"可能有等待者"
const MAYBE_WAITERS: i32 = 0x40000000;
```

---

## 结构体定义

### `struct _IO_FILE`

```rust
#[repr(C)]
pub struct _IO_FILE {
    pub flags: c_uint,                     // 文件状态位掩码
    pub rpos: *mut u8,                     // 当前读取位置
    pub rend: *mut u8,                     // 读取缓冲区末尾
    pub close: Option<unsafe extern "C" fn(*mut FILE) -> c_int>,  // 关闭操作
    pub wend: *mut u8,                     // 写缓冲区末尾
    pub wpos: *mut u8,                     // 当前写入位置
    pub mustbezero_1: *mut u8,             // 哨兵字段，必须为 null
    pub wbase: *mut u8,                    // 写缓冲区起始
    pub read: Option<unsafe extern "C" fn(*mut FILE, *mut u8, size_t) -> size_t>,  // 底层读取
    pub write: Option<unsafe extern "C" fn(*mut FILE, *const u8, size_t) -> size_t>, // 底层写入
    pub seek: Option<unsafe extern "C" fn(*mut FILE, off_t, c_int) -> off_t>,       // 底层定位
    pub buf: *mut u8,                      // 缓冲区起始地址
    pub buf_size: size_t,                  // 缓冲区总大小
    pub prev: *mut FILE,                   // 全局打开文件链表前驱
    pub next: *mut FILE,                   // 全局打开文件链表后继
    pub fd: c_int,                         // 关联的文件描述符
    pub pipe_pid: pid_t,                   // popen 子进程 PID
    pub lockcount: c_long,                 // 递归锁计数器
    pub mode: c_int,                       // 文件打开模式
    pub lock: AtomicI32,                   // 线程锁（futex 兼容）
    pub lbf: c_int,                        // 行缓冲标志
    pub cookie: *mut c_void,               // 扩展数据
    pub off: off_t,                        // 逻辑文件偏移
    pub getln_buf: *mut c_char,            // gets 缓冲区
    pub mustbezero_2: *mut c_void,         // 哨兵字段，必须为 null
    pub shend: *mut u8,                    // 扫描结束位置
    pub shlim: off_t,                      // 扫描宽度限制
    pub shcnt: off_t,                      // 已扫描字符计数
    pub prev_locked: *mut FILE,            // 线程"锁定文件"链表前驱
    pub next_locked: *mut FILE,            // 线程"锁定文件"链表后继
    pub locale: *mut __locale_struct,      // 文件关联的 locale
}
```

`[Visibility]: Internal — musl 内部实现的 FILE 实际布局，`#[repr(C)]` 保证与 C ABI 兼容。POSIX 标准定义 FILE 为不透明类型。`

**注意**: `_IO_FILE` 中的 `lock` 字段在 C 侧为 `volatile int`，在 Rust 侧使用 `AtomicI32` 实现等效语义。通过 `AtomicI32::load(Ordering::Relaxed)` 检查锁状态，通过 `AtomicI32::compare_exchange` 等操作实现 futex 兼容的锁获取/释放。

**字段分组语义**（同 C spec，此处省略详细说明，字段布局保持一致）。

---

## 全局变量

```rust
/// musl 内部实际使用的标准 I/O 流指针
#[no_mangle]
static mut __stdin_used: *mut FILE = core::ptr::null_mut();

#[no_mangle]
static mut __stdout_used: *mut FILE = core::ptr::null_mut();

#[no_mangle]
static mut __stderr_used: *mut FILE = core::ptr::null_mut();
```

`[Visibility]: Internal`

---

## 锁操作 RAII 守卫（Rust 惯用法）

C 侧的 `FLOCK`/`FUNLOCK` 宏模式对应 Rust 的 RAII 守卫：

```rust
/// 文件锁守卫 — 构造时获取锁，Drop 时释放锁
pub(crate) struct FileLockGuard {
    file: *mut FILE,
    need_unlock: bool,
}

impl FileLockGuard {
    /// 对应 C 的 FLOCK(f) 宏
    pub(crate) fn lock(file: *mut FILE) -> Self {
        let need_unlock = unsafe {
            let f = &*file;
            f.lock.load(Ordering::Relaxed) >= 0 && __lockfile(file) != 0
        };
        FileLockGuard { file, need_unlock }
    }
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        if self.need_unlock {
            unsafe { __unlockfile(self.file); }
        }
    }
}
```

---

## 内联函数 — 高速路径（替代 C 宏）

### `feof` / `ferror`

```rust
#[inline(always)]
pub unsafe fn feof(f: *const FILE) -> bool {
    ((*f).flags & FileFlags::F_EOF.bits() as c_uint) != 0
}

#[inline(always)]
pub unsafe fn ferror(f: *const FILE) -> bool {
    ((*f).flags & FileFlags::F_ERR.bits() as c_uint) != 0
}
```

### `getc_unlocked`

```rust
#[inline(always)]
pub unsafe fn getc_unlocked(f: *mut FILE) -> c_int {
    let f = &mut *f;
    if f.rpos != f.rend {
        let c = *f.rpos as c_int;
        f.rpos = f.rpos.add(1);
        c
    } else {
        __uflow(f as *mut FILE as *mut FILE)
    }
}
```

### `putc_unlocked`

```rust
#[inline(always)]
pub unsafe fn putc_unlocked(c: c_int, f: *mut FILE) -> c_int {
    let f = &mut *f;
    let ch = c as u8;
    if ch as c_int != f.lbf && f.wpos != f.wend {
        *f.wpos = ch;
        f.wpos = f.wpos.add(1);
        c
    } else {
        __overflow(f as *mut FILE as *mut FILE, ch as c_int)
    }
}
```

---

## 跨文件依赖

| 依赖符号 | 来源 | 处理方式 |
|---------|------|---------|
| `sys_read`/`sys_write`/`sys_close` | `syscall` 模块 | 底层系统调用 |
| `AtomicI32` / `Ordering` | `core::sync::atomic` | 原子操作（替代 `a_cas`/`a_inc`） |
| `futex_wait`/`futex_wake` | `futex` 模块 | futex 等待/唤醒 |
| `__locale_struct` | `locale` 模块 | FILE 关联 locale |
| `__pthread` | `pthread` 模块 | 线程锁定文件链表 |
| `bitflags` | 外部 crate | `F_*` 位标志 |

---

## 实现指南 (rusl/Rust)

- `struct _IO_FILE` 使用 `#[repr(C)]` 保证与 C ABI 完全兼容。函数指针成员用 `Option<unsafe extern "C" fn(...)>` 表示
- `lock` 字段使用 `AtomicI32` 替代 C 的 `volatile int`，通过原子操作实现 futex 兼容锁
- `FLOCK`/`FUNLOCK` 宏模式 → Rust 中使用 RAII 守卫 `FileLockGuard`，构造时加锁、析构时解锁
- 打开文件链表 → 使用 `Mutex<LinkedList>` 保护（内部模块），对外提供 `__ofl_lock`/`__ofl_unlock` 保持 ABI 兼容
- `read`/`write`/`seek`/`close` 函数指针 → 作为虚函数表的内联实现
- `flags` → `bitflags!` 宏实现 `FileFlags` 位标志类型，安全且类型化
- `pipe_pid` → `pid_t` 类型封装
- `getc_unlocked`/`putc_unlocked` → `#[inline(always)]` 函数，快速路径生成高效代码
- `mustbezero_1`/`mustbezero_2` → 保持为 `*mut u8`/`*mut c_void` 类型哨兵字段，用于 ABI 检测
- 注意：`FILE` 是 `_IO_FILE` 的类型别名（`pub type FILE = _IO_FILE;`），对外 API 使用 `*mut FILE` 指针