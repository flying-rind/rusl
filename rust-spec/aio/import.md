# aio 模块 — 外部依赖导入

本文件记录 `rusl-aio` crate 实现所需的所有外部模块 Rust 接口依赖。

---

## 类型定义（来自 rusl-aio 自身及其他 crate）

| Rust 类型 | C 对应类型 | 定义位置 | 说明 |
|-----------|-----------|----------|------|
| `aiocb` | `struct aiocb` | `rusl_aio::aiocb` | AIO 控制块，包含文件描述符、缓冲区、偏移、字节数、通知方式、状态等字段 |
| `timespec` | `struct timespec` | `rusl_time::timespec` | POSIX 时间结构，包含 `tv_sec: i64` 和 `tv_nsec: i64` |
| `sigevent` | `struct sigevent` | `rusl_signal::sigevent` | 异步 I/O 完成通知结构，支持无通知/信号/线程三种通知方式 |
| `siginfo_t` | `siginfo_t` | `rusl_signal::siginfo_t` | 信号信息结构 |
| `sigset_t` | `sigset_t` | `rusl_signal::sigset_t` | 信号集类型 |
| `sem_t` | `sem_t` | `rusl_pthread::sem_t` | POSIX 信号量 |

在 rust-spec 中，这些类型通过相应 crate 的 `extern "C"` FFI 接口暴露，保持与 C 的内存布局完全一致。

---

## 来自 `rusl-stdlib` 的内存分配接口

| C 接口 | Rust extern "C" 签名 | 使用文件 |
|--------|---------------------|----------|
| `malloc` | `extern "C" fn malloc(size: usize) -> *mut c_void;` | `lio_listio`, `aio` |
| `free` | `extern "C" fn free(ptr: *mut c_void);` | `lio_listio`, `aio` |
| `calloc` | `extern "C" fn calloc(n: usize, size: usize) -> *mut c_void;` | `aio` |
| `realloc` | `extern "C" fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;` | `aio` |

在 rusl 内部实现中，内存分配函数可直接使用 `rusl_stdlib` 导出的 `malloc`/`free`/`calloc`/`realloc`，或通过自定义分配器实现（如使用固定的预分配缓冲区）。

---

## 来自 `rusl-unistd` 的文件 I/O 接口

| C 接口 | Rust extern "C" 签名 | 使用文件 |
|--------|---------------------|----------|
| `read` | `extern "C" fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;` | `aio` |
| `write` | `extern "C" fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;` | `aio` |
| `pread` | `extern "C" fn pread(fd: c_int, buf: *mut c_void, size: usize, ofs: i64) -> isize;` | `aio` |
| `pwrite` | `extern "C" fn pwrite(fd: c_int, buf: *const c_void, size: usize, ofs: i64) -> isize;` | `aio` |
| `fsync` | `extern "C" fn fsync(fd: c_int) -> c_int;` | `aio` |
| `fdatasync` | `extern "C" fn fdatasync(fd: c_int) -> c_int;` | `aio` |
| `lseek` | `extern "C" fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;` | `aio` |
| `fcntl` | `extern "C" fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;` | `aio` |
| `close` | `extern "C" fn close(fd: c_int) -> c_int;` | 通过 `__aio_close` 间接使用 |
| `getpid` | `extern "C" fn getpid() -> c_int;` | `lio_listio`, `aio` |
| `getuid` | `extern "C" fn getuid() -> c_uint;` | `lio_listio`, `aio` |

---

## 来自 `rusl-string` 的内存操作接口

| C 接口 | Rust extern "C" 签名 | 使用文件 |
|--------|---------------------|----------|
| `memcpy` | `extern "C" fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;` | `lio_listio` |

---

## 来自 `rusl-pthread` 的线程管理接口

| C 接口 | Rust extern "C" 签名 | 使用文件 |
|--------|---------------------|----------|
| `pthread_create` | `extern "C" fn pthread_create(thr: *mut pthread_t, attr: *const pthread_attr_t, start: extern "C" fn(*mut c_void) -> *mut c_void, arg: *mut c_void) -> c_int;` | `lio_listio`, `aio` |
| `pthread_cancel` | `extern "C" fn pthread_cancel(thr: pthread_t) -> c_int;` | `aio` |
| `pthread_attr_init` | `extern "C" fn pthread_attr_init(attr: *mut pthread_attr_t) -> c_int;` | `lio_listio`, `aio` |
| `pthread_attr_setstacksize` | `extern "C" fn pthread_attr_setstacksize(attr: *mut pthread_attr_t, size: usize) -> c_int;` | `lio_listio`, `aio` |
| `pthread_attr_setguardsize` | `extern "C" fn pthread_attr_setguardsize(attr: *mut pthread_attr_t, size: usize) -> c_int;` | `lio_listio`, `aio` |
| `pthread_attr_setdetachstate` | `extern "C" fn pthread_attr_setdetachstate(attr: *mut pthread_attr_t, state: c_int) -> c_int;` | `lio_listio`, `aio` |
| `pthread_mutex_lock` | `extern "C" fn pthread_mutex_lock(m: *mut pthread_mutex_t) -> c_int;` | `aio` |
| `pthread_mutex_unlock` | `extern "C" fn pthread_mutex_unlock(m: *mut pthread_mutex_t) -> c_int;` | `aio` |
| `pthread_mutex_init` | `extern "C" fn pthread_mutex_init(m: *mut pthread_mutex_t, attr: *const pthread_mutexattr_t) -> c_int;` | `aio` |
| `pthread_cond_wait` | `extern "C" fn pthread_cond_wait(cond: *mut pthread_cond_t, m: *mut pthread_mutex_t) -> c_int;` | `aio` |
| `pthread_cond_broadcast` | `extern "C" fn pthread_cond_broadcast(cond: *mut pthread_cond_t) -> c_int;` | `aio` |
| `pthread_cond_init` | `extern "C" fn pthread_cond_init(cond: *mut pthread_cond_t, attr: *const pthread_condattr_t) -> c_int;` | `aio` |
| `pthread_rwlock_rdlock` | `extern "C" fn pthread_rwlock_rdlock(rw: *mut pthread_rwlock_t) -> c_int;` | `aio` |
| `pthread_rwlock_wrlock` | `extern "C" fn pthread_rwlock_wrlock(rw: *mut pthread_rwlock_t) -> c_int;` | `aio` |
| `pthread_rwlock_unlock` | `extern "C" fn pthread_rwlock_unlock(rw: *mut pthread_rwlock_t) -> c_int;` | `aio` |
| `pthread_rwlock_init` | `extern "C" fn pthread_rwlock_init(rw: *mut pthread_rwlock_t, attr: *const pthread_rwlockattr_t) -> c_int;` | `aio` |
| `pthread_sigmask` | `extern "C" fn pthread_sigmask(how: c_int, set: *const sigset_t, old: *mut sigset_t) -> c_int;` | `lio_listio`, `aio` |
| `pthread_cleanup_push` | `extern "C" fn pthread_cleanup_push(routine: extern "C" fn(*mut c_void), arg: *mut c_void);` | `aio` |
| `pthread_cleanup_pop` | `extern "C" fn pthread_cleanup_pop(execute: c_int);` | `aio` |
| `pthread_testcancel` | `extern "C" fn pthread_testcancel();` | `aio_suspend` |
| `__pthread_self` | `extern "C" fn __pthread_self() -> *mut pthread;` | `aio` |

**musl 内部路径**：`pthread_cleanup_push`/`pthread_cleanup_pop` 是编译器扩展宏而非函数，在 Rust 中需要使用等效的 scope guard 模式（`Drop` trait）或直接内联展开，同时须保持线程取消点的语义正确性。

---

## 来自 `rusl-pthread` 线程常量的 Rust 映射

| C 常量 | Rust 定义 | 说明 |
|--------|----------|------|
| `PTHREAD_CREATE_DETACHED` | `pub const PTHREAD_CREATE_DETACHED: c_int = 1;` | 创建分离线程 |
| `PTHREAD_RWLOCK_INITIALIZER` | 在 Rust 中以 `const` 结构体或初始化函数形式提供 | 读写锁静态初始化器 |

---

## 来自 `rusl-signal` 的信号处理接口

| C 接口/类型 | Rust extern "C" 签名 | 使用文件 |
|------------|---------------------|----------|
| `siginfo_t` | 结构体，由 `rusl_signal` 定义 (`repr(C)`) | `lio_listio`, `aio` |
| `struct sigevent` | 结构体，由 `rusl_signal` 定义 (`repr(C)`) | `lio_listio`, `aio` |
| `sigset_t` | 类型别名，由 `rusl_signal` 定义 | `lio_listio`, `aio` |
| `sigfillset` | `extern "C" fn sigfillset(set: *mut sigset_t) -> c_int;` | `lio_listio`, `aio` |

## 来自 `rusl-signal` 的信号常量

| C 常量 | Rust 定义 | 说明 |
|--------|----------|------|
| `SIG_BLOCK` | `pub const SIG_BLOCK: c_int = 0;` | 阻塞信号 |
| `SIG_SETMASK` | `pub const SIG_SETMASK: c_int = 2;` | 设置信号掩码 |
| `SI_ASYNCIO` | `pub const SI_ASYNCIO: c_int = -4;` | 异步 I/O 完成信号标识 |
| `SIGEV_NONE` | `pub const SIGEV_NONE: c_int = 1;` | 无通知 |
| `SIGEV_SIGNAL` | `pub const SIGEV_SIGNAL: c_int = 2;` | 信号通知 |
| `SIGEV_THREAD` | `pub const SIGEV_THREAD: c_int = 3;` | 线程通知 |

---

## 来自 `rusl-pthread` 的同步原语接口

| C 接口 | Rust extern "C" 签名 | 使用文件 |
|--------|---------------------|----------|
| `sem_t` | `repr(C)` 结构体，由 `rusl_pthread` 定义 | `aio` |
| `sem_init` | `extern "C" fn sem_init(sem: *mut sem_t, pshared: c_int, value: c_uint) -> c_int;` | `aio` |
| `sem_post` | `extern "C" fn sem_post(sem: *mut sem_t) -> c_int;` | `aio` |
| `sem_wait` | `extern "C" fn sem_wait(sem: *mut sem_t) -> c_int;` | `aio` |

---

## 来自 `rusl-time` 的时间接口

| C 接口/类型 | Rust extern "C" 签名 | 使用文件 |
|------------|---------------------|----------|
| `struct timespec` | `repr(C)` 结构体，由 `rusl_time` 定义 | `aio_suspend` |
| `clock_gettime` | `extern "C" fn clock_gettime(clk: clockid_t, ts: *mut timespec) -> c_int;` | `aio_suspend` |
| `CLOCK_MONOTONIC` | `pub const CLOCK_MONOTONIC: clockid_t = 1;` | `aio_suspend` |

---

## 来自 rusl 内部系统调用封装

| 内部接口 | Rust 调用方式 | 说明 |
|---------|-------------|------|
| `__syscall` (SYS_rt_sigqueueinfo) | `rusl_syscall::syscall(SYS_rt_sigqueueinfo, ...)` | 向线程发送信号并附带 siginfo |
| `__syscall` (SYS_futex) | `rusl_syscall::syscall(SYS_futex, ...)` | futex 系统调用（等待/唤醒） |
| `__syscall_ret` | `rusl_syscall::syscall_ret(r)` | 系统调用返回值到 libc 错误码转换 |

---

## 来自 rusl 内部原子操作

| C 接口 | Rust 等效类型/函数 | 使用文件 |
|--------|-------------------|----------|
| `a_inc` / `a_dec` | `core::sync::atomic::AtomicI32` 的 `fetch_add` / `fetch_sub` (Ordering::AcqRel) | `aio`, `aio_suspend` |
| `a_swap` | `core::sync::atomic::AtomicI32::swap(Ordering::AcqRel)` | `aio`, `aio_suspend` |
| `a_cas` | `core::sync::atomic::AtomicI32::compare_exchange(Ordering::AcqRel, Ordering::Acquire)` | `aio`, `aio_suspend` |
| `a_barrier` | `core::sync::atomic::fence(Ordering::SeqCst)` | `aio` |
| `a_store` | `core::sync::atomic::AtomicI32::store(Ordering::Release)` | `aio` |

在 rusl 内部实现中，可直接使用 Rust 标准 `core::sync::atomic` 原子类型替代 C 的 `a_*` 宏，提供更安全的原子操作语义。注意需要在 `#![no_std]` 环境下工作。

---

## 来自 rusl 内部线程/同步接口

| C 接口 | Rust 等效函数 | 使用文件 |
|--------|-------------|----------|
| `__wake` | `rusl_internal::__wake(addr: *const c_int, cnt: c_int)` — 唤醒 cnt 个在 addr 上等待的 futex 线程 | `aio` |
| `__futexwait` | `rusl_internal::__futexwait(addr: *const c_int, val: c_int, priv: c_int)` — futex 等待 | `aio` |
| `__wait` | `rusl_internal::__wait(addr: *const c_int, fut: *const c_int, val: c_int, priv: c_int)` — 条件 futex 等待 | `aio` |
| `__timedwait_cp` | `rusl_internal::__timedwait_cp(addr: *const c_int, val: c_int, clk: clockid_t, at: *const timespec, priv: c_int)` — 带超时和取消点的 futex 等待 | `aio`, `aio_suspend` |
| `__pthread_self` | `extern "C" fn __pthread_self() -> *mut pthread;` (来自 `rusl_pthread`) | `aio` |

## 来自 rusl 内部的 futex 常量

| C 常量 | Rust 定义 | 说明 |
|--------|----------|------|
| `FUTEX_WAKE` | `pub const FUTEX_WAKE: c_int = 129;` | futex 唤醒操作 |
| `FUTEX_WAIT` | `pub const FUTEX_WAIT: c_int = 128;` | futex 等待操作 |
| `FUTEX_PRIVATE` | `pub const FUTEX_PRIVATE: c_int = 128;` | 进程私有 futex 标志 |

---

## 来自 rusl 内部辅助接口

| C 接口 | Rust 等效接口 | 使用文件 |
|--------|-------------|----------|
| `__getauxval` | `rusl_internal::__getauxval(AT_MINSIGSTKSZ)` — 获取辅助向量值 | `aio` |
| `AT_MINSIGSTKSZ` | `pub const AT_MINSIGSTKSZ: c_ulong = 51;` | `aio` |
| `MINSIGSTKSZ` | `pub const MINSIGSTKSZ: usize = 2048;` — 最小备用信号栈大小 | `aio` |
| `PAGE_SIZE` | `pub const PAGE_SIZE: usize = 4096;` — 页面大小（Linux x86_64） | `lio_listio` |

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  上述所有来自 rusl-stdlib、rusl-unistd、rusl-string、rusl-pthread、rusl-signal、rusl-time 的 extern "C" 导出接口
  rusl-syscall 模块提供的系统调用封装
  rusl-internal 模块提供的 futex 等待、唤醒、原子操作辅助函数
Predefined Macros/Crates:
  core::ffi — Rust 核心库的 FFI 类型 (c_int, c_uint, c_char, c_void, c_ulong)
  core::sync::atomic — Rust 核心库的原子类型和操作 (AtomicI32, Ordering)
  rusl_stdlib — rusl 内存分配接口 crate
  rusl_unistd — rusl 基本 I/O 和进程管理接口 crate
  rusl_string — rusl 字符串和内存操作接口 crate
  rusl_pthread — rusl POSIX 线程和同步接口 crate
  rusl_signal — rusl 信号处理接口 crate
  rusl_time — rusl 时间接口 crate
  rusl_syscall — rusl 系统调用封装模块
  rusl_internal — rusl 内部辅助模块

[GUARANTEE]
Exported Interface:
  本文件仅记录依赖，不对外导出任何符号
