# thread 模块 — 对外导出 API 汇总

本文件记录 `rusl-thread` crate 的所有对外导出接口（由 `<pthread.h>`、`<threads.h>` 或 `<semaphore.h>` 声明），以及对应的 Rust `extern "C"` ABI 设计。所有符号均须保持与 C ABI 完全兼容。

---

## 公共数据类型（用户直接可见）

以下类型为 opaque 类型，用户不应直接访问其内部字段。使用 `#[repr(C)]` 保证与 C 的内存布局一致。

```rust
use core::ffi::{c_int, c_uint, c_void, c_char};
use core::ffi::c_ulong;

/// 线程标识符（指向内部 __pthread 结构体的指针）
/// musl 中为 `struct __pthread *`，在 Rust 中以 opaque 指针表示
pub type pthread_t = *mut c_void;

/// 线程属性对象
/// musl 中为含 14 个 unsigned 的 union，56 字节对齐至 unsigned long long
#[repr(C)]
pub struct pthread_attr_t {
    _opaque: [c_ulong; 7], // 56 字节，对齐至 8
}

/// 互斥锁
/// C 中使用 `PTHREAD_MUTEX_INITIALIZER = {{{0}}}` 零初始化
#[repr(C)]
pub struct pthread_mutex_t {
    _opaque: [c_ulong; 5], // 40 字节
}

/// 互斥锁属性对象
/// musl 中为单个 unsigned int
#[repr(C)]
pub struct pthread_mutexattr_t {
    __attr: c_uint,
}

/// 读写锁
/// C 中使用 `PTHREAD_RWLOCK_INITIALIZER = {{{0}}}` 零初始化
#[repr(C)]
pub struct pthread_rwlock_t {
    _opaque: [c_ulong; 7], // 56 字节
}

/// 读写锁属性对象
/// musl 中为 `unsigned __attr[2]`
#[repr(C)]
pub struct pthread_rwlockattr_t {
    __attr: [c_uint; 2], // 8 字节
}

/// 条件变量
#[repr(C)]
pub struct pthread_cond_t {
    _opaque: [c_ulong; 6], // 48 字节
}

/// 条件变量属性对象
/// musl 中为单个 unsigned int
#[repr(C)]
pub struct pthread_condattr_t {
    __attr: c_uint,
}

/// 屏障
#[repr(C)]
pub struct pthread_barrier_t {
    _opaque: [c_ulong; 4], // 32 字节
}

/// 屏障属性对象
/// musl 中为单个 unsigned int
#[repr(C)]
pub struct pthread_barrierattr_t {
    __attr: c_uint,
}

/// 自旋锁
/// musl 中为 `volatile int`
#[repr(C)]
pub struct pthread_spinlock_t {
    __lock: c_int,
}

/// 一次性初始化控制变量
/// C 中使用 `PTHREAD_ONCE_INIT = 0` 零初始化
pub type pthread_once_t = c_int;

/// 线程局部存储键
/// musl 中为 `unsigned`
pub type pthread_key_t = c_uint;

/// 信号量
/// sem_t 内部含 volatile int 数组和等待者计数
#[repr(C)]
pub struct sem_t {
    _opaque: [c_ulong; 4], // 32 字节
}

/// 时钟标识符（定义于 <time.h>，thread 模块引用）
pub type clockid_t = c_int;

/// 调度参数（定义于 <sched.h>）
#[repr(C)]
pub struct sched_param {
    pub sched_priority: c_int,
}

/// timespec 时间结构（定义于 <time.h>，提供时间参数）
/// rusl-time crate 提供，此处声明引用
#[repr(C)]
pub struct timespec {
    pub tv_sec: i64,  // time_t (64-bit on modern systems)
    pub tv_nsec: i64, // long
}

/// 信号集（定义于 <signal.h>，提供信号掩码参数）
/// musl 中为 `unsigned long __bits[128/(8*sizeof(long))]`
/// 在 x86_64 上为 `unsigned long __bits[2]`
#[repr(C)]
pub struct sigset_t {
    pub __bits: [c_ulong; 16], // 128 字节，兼容最大信号数
}

/// C11 线程类型
/// `thrd_t` 在 C 中为 `struct __pthread *`，与 `pthread_t` 相同
pub type thrd_t = pthread_t;

/// C11 线程入口函数类型
pub type thrd_start_t = Option<unsafe extern "C" fn(*mut c_void) -> c_int>;

/// C11 线程特定存储键
pub type tss_t = c_uint;

/// C11 TSS 析构函数类型
pub type tss_dtor_t = Option<unsafe extern "C" fn(*mut c_void)>;

/// C11 一次性执行标志
pub type once_flag = c_int;

/// C11 条件变量（与 pthread_cond_t 相同）
pub type cnd_t = pthread_cond_t;

/// C11 互斥锁（与 pthread_mutex_t 相同）
pub type mtx_t = pthread_mutex_t;
```

---

## 公共常量（用户直接可见）

### 线程属性常量

```rust
pub const PTHREAD_CREATE_JOINABLE: core::ffi::c_int = 0;
pub const PTHREAD_CREATE_DETACHED: core::ffi::c_int = 1;
pub const PTHREAD_INHERIT_SCHED: core::ffi::c_int = 0;
pub const PTHREAD_EXPLICIT_SCHED: core::ffi::c_int = 1;
pub const PTHREAD_SCOPE_SYSTEM: core::ffi::c_int = 0;
pub const PTHREAD_SCOPE_PROCESS: core::ffi::c_int = 1;
pub const PTHREAD_STACK_MIN: usize = 2048;
```

### 互斥锁常量

```rust
pub const PTHREAD_MUTEX_NORMAL: core::ffi::c_int = 0;
pub const PTHREAD_MUTEX_DEFAULT: core::ffi::c_int = 0;
pub const PTHREAD_MUTEX_RECURSIVE: core::ffi::c_int = 1;
pub const PTHREAD_MUTEX_ERRORCHECK: core::ffi::c_int = 2;
pub const PTHREAD_MUTEX_STALLED: core::ffi::c_int = 0;
pub const PTHREAD_MUTEX_ROBUST: core::ffi::c_int = 1;
pub const PTHREAD_PRIO_NONE: core::ffi::c_int = 0;
pub const PTHREAD_PRIO_INHERIT: core::ffi::c_int = 1;
pub const PTHREAD_PRIO_PROTECT: core::ffi::c_int = 2;
```

`PTHREAD_MUTEX_INITIALIZER` 在 C 中为 `{{{0}}}`，Rust 中应通过 `const` 零值构造：
```rust
// 由于 Rust const 泛型限制，提供零值常量供静态初始化使用
// 实际可用 core::mem::zeroed() 或在 static 初始化时使用零值表达式
```

### 取消常量

```rust
pub const PTHREAD_CANCEL_ENABLE: core::ffi::c_int = 0;
pub const PTHREAD_CANCEL_DISABLE: core::ffi::c_int = 1;
pub const PTHREAD_CANCEL_MASKED: core::ffi::c_int = 2;
pub const PTHREAD_CANCEL_DEFERRED: core::ffi::c_int = 0;
pub const PTHREAD_CANCEL_ASYNCHRONOUS: core::ffi::c_int = 1;
```

`PTHREAD_CANCELED` 为 `((void *)-1)`，Rust 中以指针常量表示：
```rust
pub const PTHREAD_CANCELED: *mut core::ffi::c_void = (-1isize) as *mut core::ffi::c_void;
```

### 通用常量

```rust
pub const PTHREAD_ONCE_INIT: core::ffi::c_int = 0;
pub const PTHREAD_PROCESS_PRIVATE: core::ffi::c_int = 0;
pub const PTHREAD_PROCESS_SHARED: core::ffi::c_int = 1;
pub const PTHREAD_NULL: *mut core::ffi::c_void = core::ptr::null_mut();
pub const PTHREAD_KEYS_MAX: core::ffi::c_int = 128;
pub const PTHREAD_DESTRUCTOR_ITERATIONS: core::ffi::c_int = 4;
pub const SIGCANCEL: core::ffi::c_int = 33;
pub const SIGSYNCCALL: core::ffi::c_int = 34;
```

`PTHREAD_RWLOCK_INITIALIZER` 在 C 中为 `{{{0}}}`，同理使用零值。

### 信号量常量

```rust
pub const SEM_VALUE_MAX: core::ffi::c_int = 0x7FFFFFFF;
pub const SEM_NSEMS_MAX: core::ffi::c_int = 256;
pub const SEM_FAILED: *mut sem_t = core::ptr::null_mut();
```

### C11 线程常量

```rust
pub const thrd_success: core::ffi::c_int = 0;
pub const thrd_busy: core::ffi::c_int = 1;
pub const thrd_error: core::ffi::c_int = 2;
pub const thrd_nomem: core::ffi::c_int = 3;
pub const thrd_timedout: core::ffi::c_int = 4;
pub const mtx_plain: core::ffi::c_int = 0;
pub const mtx_recursive: core::ffi::c_int = 1;
pub const mtx_timed: core::ffi::c_int = 2;
pub const ONCE_FLAG_INIT: core::ffi::c_int = 0;
pub const TSS_DTOR_ITERATIONS: core::ffi::c_int = 4;
```

---

## 1. 线程属性 (pthread_attr) API

### 1.1 属性初始化与销毁

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_attr_init` | `pub extern "C" fn pthread_attr_init(a: *mut pthread_attr_t) -> c_int;` | 初始化线程属性对象为默认值 |
| `pthread_attr_destroy` | `pub extern "C" fn pthread_attr_destroy(a: *mut pthread_attr_t) -> c_int;` | 销毁线程属性对象（空操作） |

### 1.2 属性获取 (Getter)

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_attr_getdetachstate` | `pub extern "C" fn pthread_attr_getdetachstate(a: *const pthread_attr_t, state: *mut c_int) -> c_int;` | 获取分离状态 |
| `pthread_attr_getguardsize` | `pub extern "C" fn pthread_attr_getguardsize(a: *const pthread_attr_t, size: *mut usize) -> c_int;` | 获取守护页大小 |
| `pthread_attr_getinheritsched` | `pub extern "C" fn pthread_attr_getinheritsched(a: *const pthread_attr_t, inherit: *mut c_int) -> c_int;` | 获取调度继承策略 |
| `pthread_attr_getschedparam` | `pub extern "C" fn pthread_attr_getschedparam(a: *const pthread_attr_t, param: *mut sched_param) -> c_int;` | 获取调度参数 |
| `pthread_attr_getschedpolicy` | `pub extern "C" fn pthread_attr_getschedpolicy(a: *const pthread_attr_t, policy: *mut c_int) -> c_int;` | 获取调度策略 |
| `pthread_attr_getscope` | `pub extern "C" fn pthread_attr_getscope(a: *const pthread_attr_t, scope: *mut c_int) -> c_int;` | 获取竞争范围 |
| `pthread_attr_getstack` | `pub extern "C" fn pthread_attr_getstack(a: *const pthread_attr_t, addr: *mut *mut c_void, size: *mut usize) -> c_int;` | 获取栈地址和大小 |
| `pthread_attr_getstacksize` | `pub extern "C" fn pthread_attr_getstacksize(a: *const pthread_attr_t, size: *mut usize) -> c_int;` | 获取栈大小 |

### 1.3 属性设置 (Setter)

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_attr_setdetachstate` | `pub extern "C" fn pthread_attr_setdetachstate(a: *mut pthread_attr_t, state: c_int) -> c_int;` | 设置分离状态 |
| `pthread_attr_setguardsize` | `pub extern "C" fn pthread_attr_setguardsize(a: *mut pthread_attr_t, size: usize) -> c_int;` | 设置守护页大小 |
| `pthread_attr_setinheritsched` | `pub extern "C" fn pthread_attr_setinheritsched(a: *mut pthread_attr_t, inherit: c_int) -> c_int;` | 设置调度继承策略 |
| `pthread_attr_setschedparam` | `pub extern "C" fn pthread_attr_setschedparam(a: *mut pthread_attr_t, param: *const sched_param) -> c_int;` | 设置调度参数 |
| `pthread_attr_setschedpolicy` | `pub extern "C" fn pthread_attr_setschedpolicy(a: *mut pthread_attr_t, policy: c_int) -> c_int;` | 设置调度策略 |
| `pthread_attr_setscope` | `pub extern "C" fn pthread_attr_setscope(a: *mut pthread_attr_t, scope: c_int) -> c_int;` | 设置竞争范围 |
| `pthread_attr_setstack` | `pub extern "C" fn pthread_attr_setstack(a: *mut pthread_attr_t, addr: *mut c_void, size: usize) -> c_int;` | 设置栈地址和大小 |
| `pthread_attr_setstacksize` | `pub extern "C" fn pthread_attr_setstacksize(a: *mut pthread_attr_t, size: usize) -> c_int;` | 设置栈大小 |

### 1.4 其他属性类型获取

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_barrierattr_getpshared` | `pub extern "C" fn pthread_barrierattr_getpshared(a: *const pthread_barrierattr_t, pshared: *mut c_int) -> c_int;` | 获取 barrier 进程共享标志 |
| `pthread_condattr_getclock` | `pub extern "C" fn pthread_condattr_getclock(a: *const pthread_condattr_t, clk: *mut clockid_t) -> c_int;` | 获取条件变量时钟 |
| `pthread_condattr_getpshared` | `pub extern "C" fn pthread_condattr_getpshared(a: *const pthread_condattr_t, pshared: *mut c_int) -> c_int;` | 获取条件变量进程共享标志 |
| `pthread_mutexattr_getprotocol` | `pub extern "C" fn pthread_mutexattr_getprotocol(a: *const pthread_mutexattr_t, protocol: *mut c_int) -> c_int;` | 获取互斥锁优先级协议 |
| `pthread_mutexattr_getpshared` | `pub extern "C" fn pthread_mutexattr_getpshared(a: *const pthread_mutexattr_t, pshared: *mut c_int) -> c_int;` | 获取互斥锁进程共享标志 |
| `pthread_mutexattr_getrobust` | `pub extern "C" fn pthread_mutexattr_getrobust(a: *const pthread_mutexattr_t, robust: *mut c_int) -> c_int;` | 获取互斥锁健壮性 |
| `pthread_mutexattr_gettype` | `pub extern "C" fn pthread_mutexattr_gettype(a: *const pthread_mutexattr_t, type_: *mut c_int) -> c_int;` | 获取互斥锁类型 |
| `pthread_rwlockattr_getpshared` | `pub extern "C" fn pthread_rwlockattr_getpshared(a: *const pthread_rwlockattr_t, pshared: *mut c_int) -> c_int;` | 获取读写锁进程共享标志 |

### 1.5 GNU 扩展（需 `_GNU_SOURCE`）

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_getattr_default_np` | `pub extern "C" fn pthread_getattr_default_np(attrp: *mut pthread_attr_t) -> c_int;` | 获取进程全局默认线程属性 |
| `pthread_setattr_default_np` | `pub extern "C" fn pthread_setattr_default_np(attrp: *const pthread_attr_t) -> c_int;` | 设置进程全局默认线程属性 |
| `pthread_getattr_np` | `pub extern "C" fn pthread_getattr_np(t: pthread_t, a: *mut pthread_attr_t) -> c_int;` | 从已存在线程获取实际属性 |

---

## 2. 互斥锁 (pthread_mutex) API

### 2.1 互斥锁属性操作

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_mutexattr_init` | `pub extern "C" fn pthread_mutexattr_init(a: *mut pthread_mutexattr_t) -> c_int;` | 初始化互斥锁属性对象 |
| `pthread_mutexattr_destroy` | `pub extern "C" fn pthread_mutexattr_destroy(a: *mut pthread_mutexattr_t) -> c_int;` | 销毁互斥锁属性对象（空操作） |
| `pthread_mutexattr_settype` | `pub extern "C" fn pthread_mutexattr_settype(a: *mut pthread_mutexattr_t, type_: c_int) -> c_int;` | 设置互斥锁类型 |
| `pthread_mutexattr_setpshared` | `pub extern "C" fn pthread_mutexattr_setpshared(a: *mut pthread_mutexattr_t, pshared: c_int) -> c_int;` | 设置进程共享标志 |
| `pthread_mutexattr_setrobust` | `pub extern "C" fn pthread_mutexattr_setrobust(a: *mut pthread_mutexattr_t, robust: c_int) -> c_int;` | 设置健壮性标志 |
| `pthread_mutexattr_setprotocol` | `pub extern "C" fn pthread_mutexattr_setprotocol(a: *mut pthread_mutexattr_t, protocol: c_int) -> c_int;` | 设置优先级协议 |

### 2.2 互斥锁生命周期

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_mutex_init` | `pub extern "C" fn pthread_mutex_init(m: *mut pthread_mutex_t, a: *const pthread_mutexattr_t) -> c_int;` | 初始化互斥锁 |
| `pthread_mutex_destroy` | `pub extern "C" fn pthread_mutex_destroy(mutex: *mut pthread_mutex_t) -> c_int;` | 销毁互斥锁 |

### 2.3 互斥锁同步操作

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_mutex_lock` | `pub extern "C" fn pthread_mutex_lock(m: *mut pthread_mutex_t) -> c_int;` | 阻塞加锁 |
| `pthread_mutex_trylock` | `pub extern "C" fn pthread_mutex_trylock(m: *mut pthread_mutex_t) -> c_int;` | 非阻塞尝试加锁 |
| `pthread_mutex_timedlock` | `pub extern "C" fn pthread_mutex_timedlock(m: *mut pthread_mutex_t, at: *const timespec) -> c_int;` | 带超时加锁 |
| `pthread_mutex_unlock` | `pub extern "C" fn pthread_mutex_unlock(m: *mut pthread_mutex_t) -> c_int;` | 解锁 |

### 2.4 健壮互斥锁与优先级

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_mutex_consistent` | `pub extern "C" fn pthread_mutex_consistent(m: *mut pthread_mutex_t) -> c_int;` | 将 EOWNERDEAD 互斥锁标记为一致 |
| `pthread_mutex_getprioceiling` | `pub extern "C" fn pthread_mutex_getprioceiling(m: *const pthread_mutex_t, ceiling: *mut c_int) -> c_int;` | 获取优先级天花板（始终返回 EINVAL） |
| `pthread_mutex_setprioceiling` | `pub extern "C" fn pthread_mutex_setprioceiling(m: *mut pthread_mutex_t, ceiling: c_int, old: *mut c_int) -> c_int;` | 设置优先级天花板并加锁（始终返回 EINVAL） |

---

## 3. 读写锁 (pthread_rwlock) API

### 3.1 读写锁属性

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_rwlockattr_init` | `pub extern "C" fn pthread_rwlockattr_init(a: *mut pthread_rwlockattr_t) -> c_int;` | 初始化读写锁属性对象 |
| `pthread_rwlockattr_destroy` | `pub extern "C" fn pthread_rwlockattr_destroy(a: *mut pthread_rwlockattr_t) -> c_int;` | 销毁读写锁属性对象 |
| `pthread_rwlockattr_setpshared` | `pub extern "C" fn pthread_rwlockattr_setpshared(a: *mut pthread_rwlockattr_t, pshared: c_int) -> c_int;` | 设置进程共享属性 |

### 3.2 读写锁操作

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_rwlock_init` | `pub extern "C" fn pthread_rwlock_init(rw: *mut pthread_rwlock_t, a: *const pthread_rwlockattr_t) -> c_int;` | 初始化读写锁 |
| `pthread_rwlock_destroy` | `pub extern "C" fn pthread_rwlock_destroy(rw: *mut pthread_rwlock_t) -> c_int;` | 销毁读写锁 |
| `pthread_rwlock_rdlock` | `pub extern "C" fn pthread_rwlock_rdlock(rw: *mut pthread_rwlock_t) -> c_int;` | 阻塞获取读锁 |
| `pthread_rwlock_tryrdlock` | `pub extern "C" fn pthread_rwlock_tryrdlock(rw: *mut pthread_rwlock_t) -> c_int;` | 非阻塞尝试获取读锁 |
| `pthread_rwlock_timedrdlock` | `pub extern "C" fn pthread_rwlock_timedrdlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int;` | 带超时获取读锁 |
| `pthread_rwlock_wrlock` | `pub extern "C" fn pthread_rwlock_wrlock(rw: *mut pthread_rwlock_t) -> c_int;` | 阻塞获取写锁 |
| `pthread_rwlock_trywrlock` | `pub extern "C" fn pthread_rwlock_trywrlock(rw: *mut pthread_rwlock_t) -> c_int;` | 非阻塞尝试获取写锁 |
| `pthread_rwlock_timedwrlock` | `pub extern "C" fn pthread_rwlock_timedwrlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int;` | 带超时获取写锁 |
| `pthread_rwlock_unlock` | `pub extern "C" fn pthread_rwlock_unlock(rw: *mut pthread_rwlock_t) -> c_int;` | 释放读锁或写锁 |

---

## 4. 条件变量 (pthread_cond) API

### 4.1 条件变量属性

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_condattr_init` | `pub extern "C" fn pthread_condattr_init(a: *mut pthread_condattr_t) -> c_int;` | 初始化条件变量属性 |
| `pthread_condattr_destroy` | `pub extern "C" fn pthread_condattr_destroy(a: *mut pthread_condattr_t) -> c_int;` | 销毁条件变量属性 |
| `pthread_condattr_setclock` | `pub extern "C" fn pthread_condattr_setclock(a: *mut pthread_condattr_t, clk: clockid_t) -> c_int;` | 设置等待时钟 |
| `pthread_condattr_setpshared` | `pub extern "C" fn pthread_condattr_setpshared(a: *mut pthread_condattr_t, pshared: c_int) -> c_int;` | 设置进程共享标志 |

### 4.2 条件变量操作

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_cond_init` | `pub extern "C" fn pthread_cond_init(c: *mut pthread_cond_t, a: *const pthread_condattr_t) -> c_int;` | 初始化条件变量 |
| `pthread_cond_destroy` | `pub extern "C" fn pthread_cond_destroy(c: *mut pthread_cond_t) -> c_int;` | 销毁条件变量 |
| `pthread_cond_wait` | `pub extern "C" fn pthread_cond_wait(c: *mut pthread_cond_t, m: *mut pthread_mutex_t) -> c_int;` | 无限等待条件变量 |
| `pthread_cond_timedwait` | `pub extern "C" fn pthread_cond_timedwait(c: *mut pthread_cond_t, m: *mut pthread_mutex_t, ts: *const timespec) -> c_int;` | 带超时等待条件变量 |
| `pthread_cond_signal` | `pub extern "C" fn pthread_cond_signal(c: *mut pthread_cond_t) -> c_int;` | 唤醒一个等待线程 |
| `pthread_cond_broadcast` | `pub extern "C" fn pthread_cond_broadcast(c: *mut pthread_cond_t) -> c_int;` | 唤醒所有等待线程 |

---

## 5. 屏障 (pthread_barrier) API

### 5.1 屏障属性

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_barrierattr_init` | `pub extern "C" fn pthread_barrierattr_init(a: *mut pthread_barrierattr_t) -> c_int;` | 初始化屏障属性 |
| `pthread_barrierattr_destroy` | `pub extern "C" fn pthread_barrierattr_destroy(a: *mut pthread_barrierattr_t) -> c_int;` | 销毁屏障属性 |
| `pthread_barrierattr_setpshared` | `pub extern "C" fn pthread_barrierattr_setpshared(a: *mut pthread_barrierattr_t, pshared: c_int) -> c_int;` | 设置进程共享标志 |

### 5.2 屏障操作

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_barrier_init` | `pub extern "C" fn pthread_barrier_init(b: *mut pthread_barrier_t, a: *const pthread_barrierattr_t, count: c_uint) -> c_int;` | 初始化屏障 |
| `pthread_barrier_destroy` | `pub extern "C" fn pthread_barrier_destroy(b: *mut pthread_barrier_t) -> c_int;` | 销毁屏障 |
| `pthread_barrier_wait` | `pub extern "C" fn pthread_barrier_wait(b: *mut pthread_barrier_t) -> c_int;` | 在屏障上等待 |

---

## 6. 自旋锁 (pthread_spin) API

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_spin_init` | `pub extern "C" fn pthread_spin_init(s: *mut pthread_spinlock_t, pshared: c_int) -> c_int;` | 初始化自旋锁为解锁状态 |
| `pthread_spin_destroy` | `pub extern "C" fn pthread_spin_destroy(s: *mut pthread_spinlock_t) -> c_int;` | 销毁自旋锁（空操作） |
| `pthread_spin_lock` | `pub extern "C" fn pthread_spin_lock(s: *mut pthread_spinlock_t) -> c_int;` | 忙等待获取自旋锁 |
| `pthread_spin_trylock` | `pub extern "C" fn pthread_spin_trylock(s: *mut pthread_spinlock_t) -> c_int;` | 非阻塞尝试获取自旋锁 |
| `pthread_spin_unlock` | `pub extern "C" fn pthread_spin_unlock(s: *mut pthread_spinlock_t) -> c_int;` | 释放自旋锁 |

---

## 7. 一次性初始化 (pthread_once) API

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_once` | `pub extern "C" fn pthread_once(control: *mut pthread_once_t, init: Option<unsafe extern "C" fn()>) -> c_int;` | 多线程安全的一次性初始化 |

---

## 8. 线程生命周期 API

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_create` | `pub extern "C" fn pthread_create(t: *mut pthread_t, a: *const pthread_attr_t, f: Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>, arg: *mut c_void) -> c_int;` | 创建新线程 |
| `pthread_exit` | `pub extern "C" fn pthread_exit(retval: *mut c_void);` | 终止当前线程 |
| `pthread_join` | `pub extern "C" fn pthread_join(t: pthread_t, res: *mut *mut c_void) -> c_int;` | 等待线程终止并获取返回值 |
| `pthread_detach` | `pub extern "C" fn pthread_detach(t: pthread_t) -> c_int;` | 分离线程（回收资源不等待） |

---

## 9. 线程取消 API

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_cancel` | `pub extern "C" fn pthread_cancel(t: pthread_t) -> c_int;` | 向目标线程发送取消请求 |
| `pthread_testcancel` | `pub extern "C" fn pthread_testcancel();` | 显式取消点 |
| `pthread_setcancelstate` | `pub extern "C" fn pthread_setcancelstate(state: c_int, oldstate: *mut c_int) -> c_int;` | 设置线程取消启用/禁用状态 |
| `pthread_setcanceltype` | `pub extern "C" fn pthread_setcanceltype(type_: c_int, oldtype: *mut c_int) -> c_int;` | 设置线程取消类型（延迟/异步） |

---

## 10. 线程调度 API

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_getschedparam` | `pub extern "C" fn pthread_getschedparam(t: pthread_t, policy: *mut c_int, param: *mut sched_param) -> c_int;` | 获取线程调度策略和优先级 |
| `pthread_setschedparam` | `pub extern "C" fn pthread_setschedparam(t: pthread_t, policy: c_int, param: *const sched_param) -> c_int;` | 设置线程调度策略和优先级 |
| `pthread_setschedprio` | `pub extern "C" fn pthread_setschedprio(t: pthread_t, prio: c_int) -> c_int;` | 设置线程优先级 |
| `pthread_getconcurrency` | `pub extern "C" fn pthread_getconcurrency() -> c_int;` | 获取并发级别（废弃，始终返回 0） |
| `pthread_setconcurrency` | `pub extern "C" fn pthread_setconcurrency(val: c_int) -> c_int;` | 设置并发级别（废弃） |
| `pthread_getcpuclockid` | `pub extern "C" fn pthread_getcpuclockid(t: pthread_t, clk: *mut clockid_t) -> c_int;` | 获取线程 CPU 时钟 ID |

---

## 11. 线程标识 API

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_self` | `pub extern "C" fn pthread_self() -> pthread_t;` | 获取当前线程标识符 |
| `pthread_equal` | `pub extern "C" fn pthread_equal(a: pthread_t, b: pthread_t) -> c_int;` | 比较两个线程标识符 |

---

## 12. 线程信号 API

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_kill` | `pub extern "C" fn pthread_kill(t: pthread_t, sig: c_int) -> c_int;` | 向指定线程发送信号 |
| `pthread_sigmask` | `pub extern "C" fn pthread_sigmask(how: c_int, set: *const sigset_t, old: *mut sigset_t) -> c_int;` | 检查/修改线程信号掩码 |

---

## 13. 清理处理 API

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `_pthread_cleanup_push` | `pub extern "C" fn _pthread_cleanup_push(cb: *mut c_void, f: Option<unsafe extern "C" fn(*mut c_void)>, x: *mut c_void);` | 清理栈 push 底层函数 |
| `_pthread_cleanup_pop` | `pub extern "C" fn _pthread_cleanup_pop(cb: *mut c_void, execute: c_int);` | 清理栈 pop 底层函数 |

`pthread_cleanup_push` 和 `pthread_cleanup_pop` 在 C 中为宏，在 Rust 中无法直接以宏形式导出为 extern "C" 符号。rusl 应导出底层函数 `_pthread_cleanup_push` 和 `_pthread_cleanup_pop`，上层 crate 可通过 Rust 宏包装提供 `pthread_cleanup_push`/`pthread_cleanup_pop` 宏。

---

## 14. Fork 处理 API

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_atfork` | `pub extern "C" fn pthread_atfork(prepare: Option<unsafe extern "C" fn()>, parent: Option<unsafe extern "C" fn()>, child: Option<unsafe extern "C" fn()>) -> c_int;` | 注册 fork 前/后回调 |

---

## 15. 线程局部存储 (TSD) API

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_key_create` | `pub extern "C" fn pthread_key_create(k: *mut pthread_key_t, dtor: Option<unsafe extern "C" fn(*mut c_void)>) -> c_int;` | 创建 TSD 键 |
| `pthread_key_delete` | `pub extern "C" fn pthread_key_delete(k: pthread_key_t) -> c_int;` | 删除 TSD 键 |
| `pthread_getspecific` | `pub extern "C" fn pthread_getspecific(k: pthread_key_t) -> *mut c_void;` | 获取当前线程的 TSD 值 |
| `pthread_setspecific` | `pub extern "C" fn pthread_setspecific(k: pthread_key_t, x: *const c_void) -> c_int;` | 设置当前线程的 TSD 值 |

---

## 16. 线程命名 API（GNU 扩展）

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `pthread_setname_np` | `pub extern "C" fn pthread_setname_np(thread: pthread_t, name: *const c_char) -> c_int;` | 设置线程名称（最多 15 字符） |
| `pthread_getname_np` | `pub extern "C" fn pthread_getname_np(thread: pthread_t, name: *mut c_char, len: usize) -> c_int;` | 获取线程名称 |

---

## 17. 信号量 API

### 17.1 匿名信号量

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `sem_init` | `pub extern "C" fn sem_init(sem: *mut sem_t, pshared: c_int, value: c_uint) -> c_int;` | 初始化匿名信号量 |
| `sem_destroy` | `pub extern "C" fn sem_destroy(sem: *mut sem_t) -> c_int;` | 销毁匿名信号量 |
| `sem_getvalue` | `pub extern "C" fn sem_getvalue(sem: *mut sem_t, valp: *mut c_int) -> c_int;` | 查询信号量当前值 |
| `sem_wait` | `pub extern "C" fn sem_wait(sem: *mut sem_t) -> c_int;` | 阻塞递减（P 操作） |
| `sem_trywait` | `pub extern "C" fn sem_trywait(sem: *mut sem_t) -> c_int;` | 非阻塞试探递减 |
| `sem_timedwait` | `pub extern "C" fn sem_timedwait(sem: *mut sem_t, at: *const timespec) -> c_int;` | 带超时的阻塞递减 |
| `sem_post` | `pub extern "C" fn sem_post(sem: *mut sem_t) -> c_int;` | 递增（V 操作） |

### 17.2 有名信号量

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `sem_open` | `pub extern "C" fn sem_open(name: *const c_char, flags: c_int, ...) -> *mut sem_t;` | 打开/创建有名信号量（可变参数函数） |
| `sem_close` | `pub extern "C" fn sem_close(sem: *mut sem_t) -> c_int;` | 关闭有名信号量 |
| `sem_unlink` | `pub extern "C" fn sem_unlink(name: *const c_char) -> c_int;` | 从系统移除有名信号量 |

---

## 18. C11 线程 (thrd/cnd/mtx/tss/call_once) API

### 18.1 一次性执行

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `call_once` | `pub extern "C" fn call_once(flag: *mut once_flag, func: Option<unsafe extern "C" fn()>);` | 确保 func 恰好执行一次 |

### 18.2 条件变量

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `cnd_init` | `pub extern "C" fn cnd_init(c: *mut cnd_t) -> c_int;` | 初始化条件变量 |
| `cnd_destroy` | `pub extern "C" fn cnd_destroy(c: *mut cnd_t);` | 销毁条件变量 |
| `cnd_wait` | `pub extern "C" fn cnd_wait(c: *mut cnd_t, m: *mut mtx_t) -> c_int;` | 在条件变量上等待（无限期） |
| `cnd_timedwait` | `pub extern "C" fn cnd_timedwait(c: *mut cnd_t, m: *mut mtx_t, ts: *const timespec) -> c_int;` | 在条件变量上等待（带超时） |
| `cnd_signal` | `pub extern "C" fn cnd_signal(c: *mut cnd_t) -> c_int;` | 唤醒一个等待线程 |
| `cnd_broadcast` | `pub extern "C" fn cnd_broadcast(c: *mut cnd_t) -> c_int;` | 唤醒所有等待线程 |

### 18.3 互斥锁

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `mtx_init` | `pub extern "C" fn mtx_init(m: *mut mtx_t, type_: c_int) -> c_int;` | 初始化互斥锁 |
| `mtx_destroy` | `pub extern "C" fn mtx_destroy(mtx: *mut mtx_t);` | 销毁互斥锁 |
| `mtx_lock` | `pub extern "C" fn mtx_lock(m: *mut mtx_t) -> c_int;` | 锁定互斥锁（阻塞） |
| `mtx_timedlock` | `pub extern "C" fn mtx_timedlock(m: *mut mtx_t, ts: *const timespec) -> c_int;` | 锁定互斥锁（带超时） |
| `mtx_trylock` | `pub extern "C" fn mtx_trylock(m: *mut mtx_t) -> c_int;` | 尝试锁定互斥锁（非阻塞） |
| `mtx_unlock` | `pub extern "C" fn mtx_unlock(mtx: *mut mtx_t) -> c_int;` | 解锁互斥锁 |

### 18.4 线程管理

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `thrd_create` | `pub extern "C" fn thrd_create(thr: *mut thrd_t, func: thrd_start_t, arg: *mut c_void) -> c_int;` | 创建新线程 |
| `thrd_exit` | `pub extern "C" fn thrd_exit(result: c_int) -> !;` | 终止当前线程（不发散） |
| `thrd_join` | `pub extern "C" fn thrd_join(t: thrd_t, res: *mut c_int) -> c_int;` | 等待线程终止并获取退出码 |
| `thrd_sleep` | `pub extern "C" fn thrd_sleep(req: *const timespec, rem: *mut timespec) -> c_int;` | 线程睡眠指定时长 |
| `thrd_yield` | `pub extern "C" fn thrd_yield();` | 让出 CPU |
| `thrd_current` | `pub extern "C" fn thrd_current() -> pthread_t;` | C11 版 pthread_self |
| `thrd_equal` | `pub extern "C" fn thrd_equal(a: pthread_t, b: pthread_t) -> c_int;` | C11 版 pthread_equal |

### 18.5 线程特定存储

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `tss_create` | `pub extern "C" fn tss_create(tss: *mut tss_t, dtor: tss_dtor_t) -> c_int;` | 创建 TSS 键 |
| `tss_delete` | `pub extern "C" fn tss_delete(key: tss_t);` | 删除 TSS 键 |
| `tss_set` | `pub extern "C" fn tss_set(k: tss_t, x: *mut c_void) -> c_int;` | 设置当前线程的 TSS 值 |
| `tss_get` | `pub extern "C" fn tss_get(k: tss_t) -> *mut c_void;` | C11 版 pthread_getspecific（弱别名） |

---

## 19. musl `__` 前缀内部符号（rusl 必须同时导出）

musl 中 `__xxx` 是主实现函数，无前缀的 POSIX 名称通过 `weak_alias` 映射。rusl 必须同时提供两者作为独立符号。

### 19.1 rwlock `__` 符号

| 内部符号 | Rust extern "C" 签名 | 对外导出符号 |
|---------|---------------------|-------------|
| `__pthread_rwlock_rdlock` | `pub extern "C" fn __pthread_rwlock_rdlock(rw: *mut pthread_rwlock_t) -> c_int;` | `pthread_rwlock_rdlock` |
| `__pthread_rwlock_tryrdlock` | `pub extern "C" fn __pthread_rwlock_tryrdlock(rw: *mut pthread_rwlock_t) -> c_int;` | `pthread_rwlock_tryrdlock` |
| `__pthread_rwlock_timedrdlock` | `pub extern "C" fn __pthread_rwlock_timedrdlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int;` | `pthread_rwlock_timedrdlock` |
| `__pthread_rwlock_wrlock` | `pub extern "C" fn __pthread_rwlock_wrlock(rw: *mut pthread_rwlock_t) -> c_int;` | `pthread_rwlock_wrlock` |
| `__pthread_rwlock_trywrlock` | `pub extern "C" fn __pthread_rwlock_trywrlock(rw: *mut pthread_rwlock_t) -> c_int;` | `pthread_rwlock_trywrlock` |
| `__pthread_rwlock_timedwrlock` | `pub extern "C" fn __pthread_rwlock_timedwrlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int;` | `pthread_rwlock_timedwrlock` |
| `__pthread_rwlock_unlock` | `pub extern "C" fn __pthread_rwlock_unlock(rw: *mut pthread_rwlock_t) -> c_int;` | `pthread_rwlock_unlock` |

### 19.2 mutex `__` 符号

| 内部符号 | Rust extern "C" 签名 | 对外导出符号 |
|---------|---------------------|-------------|
| `__pthread_mutex_lock` | `pub extern "C" fn __pthread_mutex_lock(m: *mut pthread_mutex_t) -> c_int;` | `pthread_mutex_lock` |
| `__pthread_mutex_trylock` | `pub extern "C" fn __pthread_mutex_trylock(m: *mut pthread_mutex_t) -> c_int;` | `pthread_mutex_trylock` |
| `__pthread_mutex_timedlock` | `pub extern "C" fn __pthread_mutex_timedlock(m: *mut pthread_mutex_t, at: *const timespec) -> c_int;` | `pthread_mutex_timedlock` |
| `__pthread_mutex_unlock` | `pub extern "C" fn __pthread_mutex_unlock(m: *mut pthread_mutex_t) -> c_int;` | `pthread_mutex_unlock` |

### 19.3 其他 `__` 符号

| 内部符号 | Rust extern "C" 签名 | 对外导出符号 |
|---------|---------------------|-------------|
| `__pthread_once` | `pub extern "C" fn __pthread_once(control: *mut pthread_once_t, init: Option<unsafe extern "C" fn()>) -> c_int;` | `pthread_once` |
| `__pthread_testcancel` | `pub extern "C" fn __pthread_testcancel();` | `pthread_testcancel` |
| `__pthread_setcancelstate` | `pub extern "C" fn __pthread_setcancelstate(state: c_int, oldstate: *mut c_int) -> c_int;` | `pthread_setcancelstate` |
| `__pthread_self_internal` | `pub extern "C" fn __pthread_self_internal() -> pthread_t;` | `pthread_self`, `thrd_current` |
| `__pthread_equal` | `pub extern "C" fn __pthread_equal(a: pthread_t, b: pthread_t) -> c_int;` | `pthread_equal`, `thrd_equal` |
| `__pthread_key_create` | `pub extern "C" fn __pthread_key_create(k: *mut pthread_key_t, dtor: Option<unsafe extern "C" fn(*mut c_void)>) -> c_int;` | `pthread_key_create` |
| `__pthread_key_delete` | `pub extern "C" fn __pthread_key_delete(k: pthread_key_t) -> c_int;` | `pthread_key_delete` |
| `__pthread_getspecific` | `pub extern "C" fn __pthread_getspecific(k: pthread_key_t) -> *mut c_void;` | `pthread_getspecific`, `tss_get` |

---

## 20. 内部导出符号（被其他模块使用，用户不应直接调用）

以下符号不通过标准头文件暴露给用户程序，但被 musl 其他内部模块引用。rusl 必须以 `pub extern "C"` 导出以保持链接兼容性。

| 符号 | Rust 类型 / 签名 | 说明 |
|------|-----------------|------|
| `__pthread_tsd_size` | `pub static mut __pthread_tsd_size: usize;` | TSD 数组总字节大小 |
| `__pthread_tsd_main` | `pub static mut __pthread_tsd_main: [*mut c_void; 128];` | 主线程默认 TSD 数组 |
| `__pthread_tsd_run_dtors` | `pub extern "C" fn __pthread_tsd_run_dtors();` | 线程退出时运行 TSD 析构函数 |
| `__sem_open_lockptr` | `pub static __sem_open_lockptr: *mut core::ffi::c_int;` | semtab 锁指针（指向 sem_open 哈希表守护锁的 volatile int） |
| `__fork_handler` | `pub extern "C" fn __fork_handler(who: c_int);` | fork 处理回调调度 |
| `__cancel` | `pub extern "C" fn __cancel() -> isize;` | 线程取消退出函数 |
| `__syscall_cp_c` | `pub extern "C" fn __syscall_cp_c(nr: isize, ...) -> isize;` | 取消点系统调用包装（C 实现、变长参数） |
| `__syscall_cp_asm` | `pub extern "C" fn __syscall_cp_asm(nr: isize, u: isize, v: isize, w: isize, x: isize, y: isize, z: isize) -> isize;` | 取消点系统调用汇编包装（6 参数） |

---

## 排除说明

以下类别的符号不需要在 Rust 外部 ABI 中导出：

- 所有 `static` 内部函数（已重构为 Rust `pub(crate)` 或更小可见性的私有函数）
- 内部静态变量（已重构为 Rust `static`，以模块私有可见性实现）
- `pthread_cleanup_push` / `pthread_cleanup_pop` 在 C 中是宏，以 C ABI 无法导出。rusl 导出底层函数 `_pthread_cleanup_push` / `_pthread_cleanup_pop`，上层 crate 可通过 Rust 宏模拟 C 宏行为
- `PTHREAD_MUTEX_INITIALIZER` / `PTHREAD_RWLOCK_INITIALIZER` 在 C 中是预处理器宏（`{{{0}}}`），在 Rust 中由类型自身的零值表达，无需单独导出常量
- 架构特定实现（作为 `cfg(target_arch = "...")` 的条件编译模块）

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  pthread_t / pthread_attr_t / pthread_mutex_t / pthread_mutexattr_t — rusl-thread 内部定义的 #[repr(C)] 结构体
  pthread_rwlock_t / pthread_rwlockattr_t — rusl-thread 内部定义的 #[repr(C)] 结构体
  pthread_cond_t / pthread_condattr_t — rusl-thread 内部定义的 #[repr(C)] 结构体
  pthread_barrier_t / pthread_barrierattr_t — rusl-thread 内部定义的 #[repr(C)] 结构体
  pthread_spinlock_t / pthread_once_t / pthread_key_t — rusl-thread 内部定义的类型
  sem_t — rusl-thread 内部定义的 #[repr(C)] 结构体
  sched_param — rusl-thread 内部定义的 #[repr(C)] 调度参数结构体
  timespec — rusl-time crate 提供的 #[repr(C)] 时间结构
  sigset_t — rusl-signal crate 提供的 #[repr(C)] 信号集结构体
  clockid_t — rusl-time crate 提供的时钟标识符类型
  cnd_t / mtx_t / tss_t / thrd_t / thrd_start_t / tss_dtor_t / once_flag — C11 线程类型别名
  rusl-syscall 提供的系统调用封装接口
Predefined Macros/Crates:
  core::ffi — Rust 核心库的 FFI 类型 (c_int, c_uint, c_char, c_void, c_ulong)
  core::sync::atomic — Rust 核心库的原子类型
  core::ptr — Rust 核心库的指针操作
  rusl-time — 提供 timespec 类型和时钟相关接口
  rusl-signal — 提供 sigset_t 类型和信号相关接口

[GUARANTEE]
Exported Interface:
  上述所有符号均须以 pub extern "C" 导出，保证与 C ABI 完全兼容
  musl __ 前缀内部符号必须同时导出（作为独立符号，非弱别名）
  所有 POSIX pthread_* 符号必须与 musl 同名符号具有相同的调用约定和内存布局
  C11 threads.h 符号（call_once, cnd_*, mtx_*, thrd_*, tss_*）必须与 musl 同名符号 ABI 兼容
  sem_* 信号量函数必须与 musl 同名符号 ABI 兼容
  内部导出符号（__pthread_tsd_size, __sem_open_lockptr, __fork_handler, __cancel, __syscall_cp_c, __syscall_cp_asm）必须保持与 musl 相同的符号名称和 ABI
  所有 #[repr(C)] 结构体的 size 和 alignment 必须与 musl libc 的对应类型完全一致（当前以 x86_64 Linux 为基准）
  sem_open 为可变参数函数，Rust 中以 extern "C" 可变参数签名导出，且必须在调用约定上兼容
  thrd_exit 在 C 中标记为 _Noreturn，Rust 中以 `-> !` (never 类型) 表示
