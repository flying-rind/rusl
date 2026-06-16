# thread 模块 — 外部依赖导入

本文件记录 `rusl_thread` crate 实现所需的所有外部模块 Rust 接口依赖。

---

## 类型定义（来自 `rusl_thread` 自身及其他 crate）

| Rust 类型 | C 对应类型 | 定义位置 | 说明 |
|-----------|-----------|----------|------|
| `pthread_mutex_t` | `pthread_mutex_t` | `rusl_thread::mutex` | 互斥锁类型，`#[repr(C)]` 结构体，含 `__u` union，通过 `_m_type`/`_m_lock`/`_m_waiters`/`_m_prev`/`_m_next`/`_m_count` 宏访问字段 |
| `pthread_mutexattr_t` | `pthread_mutexattr_t` | `rusl_thread::mutex_attr` | 互斥锁属性类型，`#[repr(C)]` |
| `pthread_cond_t` | `pthread_cond_t` | `rusl_thread::cond` | 条件变量类型，`#[repr(C)]` 结构体 |
| `pthread_condattr_t` | `pthread_condattr_t` | `rusl_thread::cond_attr` | 条件变量属性类型，`#[repr(C)]` |
| `pthread_rwlock_t` | `pthread_rwlock_t` | `rusl_thread::rwlock` | 读写锁类型，`#[repr(C)]` 结构体 |
| `pthread_rwlockattr_t` | `pthread_rwlockattr_t` | `rusl_thread::rwlock_attr` | 读写锁属性类型，`#[repr(C)]` |
| `pthread_barrier_t` | `pthread_barrier_t` | `rusl_thread::barrier` | 屏障类型，`#[repr(C)]` 结构体 |
| `pthread_barrierattr_t` | `pthread_barrierattr_t` | `rusl_thread::barrier_attr` | 屏障属性类型，`#[repr(C)]` |
| `pthread_spinlock_t` | `pthread_spinlock_t` | `rusl_thread::spin` | 自旋锁类型，类型别名 `c_int` |
| `pthread_once_t` | `pthread_once_t` | `rusl_thread::once` | 一次性初始化控制变量，`#[repr(C)]` |
| `pthread_key_t` | `pthread_key_t` | `rusl_thread::key` | TSD 键类型，类型别名 |
| `pthread_attr_t` | `pthread_attr_t` | `rusl_thread::attr` | 线程属性对象，`#[repr(C)]` union，通过 `_a_stacksize`/`_a_guardsize` 等宏访问字段 |
| `pthread_t` | `pthread_t` | `rusl_thread::types` | 线程标识符类型，指向 `pthread` 结构体的指针 |
| `sem_t` | `sem_t` | `rusl_thread::sem` | POSIX 信号量类型，`#[repr(C)]` 结构体 |
| `sigset_t` | `sigset_t` | `rusl_thread::types` | 信号集类型，`#[repr(C)]` |
| `timespec` | `struct timespec` | `rusl_thread::types` | POSIX 时间结构，`#[repr(C)]` `{ tv_sec: i64, tv_nsec: i64 }` |
| `sched_param` | `struct sched_param` | `rusl_thread::types` | 调度参数结构体，`#[repr(C)]` |
| `stat` | `struct stat` | `rusl_unistd::stat` | 文件状态结构体，`#[repr(C)]` |
| `pthread` | `struct pthread` | `rusl_thread::pthread` | 线程控制块，`#[repr(C)]` 结构体，含 tid, cancel, canceldisable, cancelasync, killlock, cancelbuf, detach_state, stack, stack_size, guard_size, robust_list, tsd, tsd_used 等字段 |
| `__ptcb` | `struct __ptcb` | `rusl_thread::ptcb` | 取消清理控制块，`#[repr(C)]` 结构体，含 `__f: Option<extern "C" fn(*mut c_void)>`, `__x: *mut c_void`, `__next: *mut __ptcb` |
| `__libc` | `struct __libc` | `rusl_internal::libc` | musl 全局运行时上下文，含 auxv, page_size 等字段 |

在 rust-spec 中，这些类型通过 `extern "C"` FFI 接口暴露，保持与 C 的内存布局完全一致。

---

## C11 / POSIX 类型兼容性映射

| C11 类型 | POSIX 内部类型 | Rust 映射 |
|----------|---------------|-----------|
| `cnd_t` | `pthread_cond_t` | `pub type cnd_t = pthread_cond_t;` |
| `mtx_t` | `pthread_mutex_t` | `pub type mtx_t = pthread_mutex_t;` |
| `thrd_t` | `struct __pthread *` | `pub type thrd_t = *mut pthread;` |
| `once_flag` | `int` | `pub type once_flag = c_int;` |
| `tss_t` | `unsigned` | `pub type tss_t = c_uint;` |

---

## 来自 `core::sync::atomic` 的原子操作

rusl 内部使用 Rust 标准 `core::sync::atomic` 原子类型替代 C 的 `a_*` 宏，
提供安全的原子操作语义且兼容 `#![no_std]` 环境。

| C 原子操作 | Rust 等效类型/函数 | 使用位置 |
|-----------|-------------------|----------|
| `a_cas(p, t, s)` | `AtomicI32::compare_exchange(t, s, AcqRel, Acquire)` | pthread_spin_lock, pthread_spin_trylock, pthread_once, mutex lock/trylock/timedlock/unlock, sem_post, sem_trywait, sem_timedwait, mtx_lock, mtx_trylock |
| `a_swap(p, v)` | `AtomicI32::swap(v, AcqRel)` | pthread_once, mutex unlock |
| `a_store(p, v)` | `AtomicI32::store(v, Release)` | pthread_spin_unlock, pthread_cancel, mutex setprotocol/setrobust/timedlock/unlock |
| `a_spin()` | `core::hint::spin_loop()` | pthread_spin_lock, sem_timedwait, mutex timedlock |
| `a_barrier()` | `core::sync::atomic::fence(SeqCst)` | pthread_once, pthread_cancel |
| `a_inc(p)` | `AtomicI32::fetch_add(1, Relaxed)` | sem_timedwait, mutex timedlock |
| `a_dec(p)` | `AtomicI32::fetch_sub(1, Relaxed)` | sem_timedwait, mutex timedlock |
| `a_and(p, v)` | `AtomicI32::fetch_and(v, Relaxed)` | mutex consistent |

---

## 来自 `rusl_syscall` 的系统调用接口

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `__syscall` | `rusl_syscall::syscall(nr: c_long, ...) -> c_long` | pthread_cancel, pthread_kill, pthread_sigmask, mutex 所有文件 |
| `__syscall_cp_asm` | `rusl_internal::__syscall_cp_asm(cp: *const c_void, nr: c_long, ...) -> c_long` | pthread_cancel |
| `__clock_nanosleep` | `extern "C" fn __clock_nanosleep(clk: clockid_t, flags: c_int, req: *const timespec, rem: *mut timespec) -> c_int;` | thrd_sleep |

### 系统调用号常量

| C 符号 | Rust 定义 | 说明 | 使用位置 |
|--------|----------|------|---------|
| `SYS_futex` | `pub const SYS_futex: c_long = 202;` | futex 系统调用 | pthread_once, mutex |
| `SYS_futex_time64` | `pub const SYS_futex_time64: c_long = ...;` (架构相关) | 64 位时间 futex | mutex timedlock |
| `SYS_tkill` | `pub const SYS_tkill: c_long = 200;` | 向线程发送信号 | pthread_cancel, pthread_kill |
| `SYS_close` | `pub const SYS_close: c_long = 3;` | 关闭文件描述符 | pthread_cancel |
| `SYS_rt_sigprocmask` | `pub const SYS_rt_sigprocmask: c_long = 14;` | 操作线程信号掩码 | pthread_sigmask |
| `SYS_sched_yield` | `pub const SYS_sched_yield: c_long = ...;` (架构相关) | 让出 CPU | thrd_yield |
| `SYS_get_robust_list` | `pub const SYS_get_robust_list: c_long = ...;` (架构相关) | 探测内核 robust list 支持 | mutex setrobust |
| `SYS_set_robust_list` | `pub const SYS_set_robust_list: c_long = ...;` (架构相关) | 注册线程 robust mutex 链表 | mutex trylock |
| `SYS_gettid` | `pub const SYS_gettid: c_long = 186;` | 获取内核线程 ID | synccall |
| `SYS_prctl` | `pub const SYS_prctl: c_long = 157;` (x86_64) | 进程控制操作 | pthread_getname_np, pthread_setname_np |

---

## 来自 `rusl_internal` 的 futex 同步原语接口

这些是 rusl 内部的 futex 包装函数，对外导出 `extern "C"` ABI 兼容符号。

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `__wake` | `pub extern "C" fn __wake(addr: *mut c_int, cnt: c_int, priv: c_int);` | sem_post, pthread_once, mutex unlock |
| `__wait` | `pub extern "C" fn __wait(addr: *mut c_int, waiters: *mut c_int, val: c_int, priv: c_int);` | pthread_once |
| `__timedwait` | `pub extern "C" fn __timedwait(addr: *mut c_int, val: c_int, clk: clockid_t, at: *const timespec, priv: c_int) -> c_int;` | mutex timedlock |
| `__timedwait_cp` | `pub extern "C" fn __timedwait_cp(addr: *mut c_int, val: c_int, clk: clockid_t, at: *const timespec, priv: c_int) -> c_int;` | sem_timedwait |

### Futex 常量

| C 宏 | Rust 定义 | 说明 |
|------|----------|------|
| `FUTEX_WAIT` | `pub const FUTEX_WAIT: c_int = 0;` | 等待 futex |
| `FUTEX_WAKE` | `pub const FUTEX_WAKE: c_int = 1;` | 唤醒 futex 等待者 |
| `FUTEX_LOCK_PI` | `pub const FUTEX_LOCK_PI: c_int = 6;` | PI futex 加锁 |
| `FUTEX_UNLOCK_PI` | `pub const FUTEX_UNLOCK_PI: c_int = 7;` | PI futex 解锁 |
| `FUTEX_PRIVATE` | `pub const FUTEX_PRIVATE: c_int = 128;` | 进程私有 futex 标志 |
| `FUTEX_CLOCK_REALTIME` | `pub const FUTEX_CLOCK_REALTIME: c_int = 256;` | 实时钟 futex |

---

## 来自 `rusl_thread` 内部的自旋锁接口

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `__lock` | `pub extern "C" fn __lock(l: *mut c_int);` | pthread_kill, pthread_atfork, sem_open |
| `__unlock` | `pub extern "C" fn __unlock(l: *mut c_int);` | pthread_kill, pthread_atfork, sem_open |

---

## 来自 `rusl_internal` 的线程基础设施接口

### 线程控制

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `__pthread_self` | `pub extern "C" fn __pthread_self() -> *mut pthread;` | pthread_cancel, pthread_once, pthread_setcancelstate/type, pthread_testcancel, pthread_self, mutex consistent/trylock/timedlock/unlock, pthread_key_create, pthread_getspecific, pthread_setspecific, tss_set |

**Rust 内部等效**：`__pthread_self()` 在 C 中为宏（读取 TLS 线程指针寄存器），在 Rust 中通过内联函数或 `extern "C"` 封装 TLS 访问实现。
TLS 寄存器读取在 x86_64 上为 `fs` 段寄存器偏移读取，在 Rust 中用 `#[inline]` 内联汇编或 `extern "C"` 函数封装。

### 线程生命周期

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `__pthread_create` | `pub extern "C" fn __pthread_create(thr: *mut thrd_t, attr: *mut c_void, entry: Option<extern "C" fn(*mut c_void) -> *mut c_void>, arg: *mut c_void) -> c_int;` | thrd_create |
| `__pthread_exit` | `pub extern "C" fn __pthread_exit(result: *mut c_void) -> !;` | thrd_exit |
| `__pthread_join` | `pub extern "C" fn __pthread_join(thr: thrd_t, res: *mut *mut c_void) -> c_int;` | thrd_join |

### 同步原语内部

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `__pthread_once` | `pub extern "C" fn __pthread_once(flag: *mut once_flag, func: Option<extern "C" fn()>) -> c_int;` | call_once |
| `__pthread_mutex_timedlock` | `pub extern "C" fn __pthread_mutex_timedlock(m: *mut pthread_mutex_t, at: *const timespec) -> c_int;` | mtx_timedlock, mutex lock |
| `__pthread_mutex_trylock` | `pub extern "C" fn __pthread_mutex_trylock(m: *mut pthread_mutex_t) -> c_int;` | mtx_trylock, mutex lock, mutex timedlock |
| `__pthread_mutex_unlock` | `pub extern "C" fn __pthread_mutex_unlock(m: *mut pthread_mutex_t) -> c_int;` | mtx_unlock |
| `__pthread_cond_timedwait` | `pub extern "C" fn __pthread_cond_timedwait(c: *mut pthread_cond_t, m: *mut pthread_mutex_t, at: *const timespec) -> c_int;` | cnd_timedwait |
| `__private_cond_signal` | `pub extern "C" fn __private_cond_signal(c: *mut pthread_cond_t, n: c_int) -> c_int;` | cnd_signal, cnd_broadcast |

### TSD 内部

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `__pthread_key_create` | `pub extern "C" fn __pthread_key_create(key: *mut tss_t, dtor: Option<extern "C" fn(*mut c_void)>) -> c_int;` | tss_create |
| `__pthread_key_delete` | `pub extern "C" fn __pthread_key_delete(key: tss_t);` | tss_delete |

### 取消相关

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `__testcancel` | `pub extern "C" fn __testcancel();` | pthread_testcancel, pthread_cancel |
| `__do_cleanup_push` | `pub extern "C" fn __do_cleanup_push(cb: *mut __ptcb);` | pthread_cleanup_push |
| `__do_cleanup_pop` | `pub extern "C" fn __do_cleanup_pop(cb: *mut __ptcb);` | pthread_cleanup_push |

### PTC 锁

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `__acquire_ptc` | `pub extern "C" fn __acquire_ptc();` | pthread_attr_init, pthread_setattr_default_np |
| `__release_ptc` | `pub extern "C" fn __release_ptc();` | pthread_attr_init, pthread_setattr_default_np |
| `__inhibit_ptc` | `pub extern "C" fn __inhibit_ptc();` | pthread_setattr_default_np |

### 线程列表锁

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `__tl_lock` | `pub extern "C" fn __tl_lock();` | pthread_key_create |
| `__tl_unlock` | `pub extern "C" fn __tl_unlock();` | pthread_key_create |

### VM 锁

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `__vm_lock` | `pub extern "C" fn __vm_lock();` | mutex unlock |
| `__vm_unlock` | `pub extern "C" fn __vm_unlock();` | mutex unlock |
| `__vm_wait` | `pub extern "C" fn __vm_wait();` | mutex destroy |

---

## 来自 `rusl_stdlib` / `rusl_malloc` 的内存分配接口

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `__libc_malloc` | `extern "C" fn __libc_malloc(size: usize) -> *mut c_void;` | pthread_atfork |
| `calloc` | `extern "C" fn calloc(nmemb: usize, size: usize) -> *mut c_void;` | sem_open |

在 rusl 内部实现中，内存分配可直接使用 `rusl_stdlib` 导出的 `malloc`/`calloc`/`free`。
对于循环依赖敏感的路径（如 `pthread_atfork`），使用 `__libc_malloc` 绕过 libc 内部的 malloc 包装。

---

## 来自 `rusl_thread` 内部的信号管理接口

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `__block_app_sigs` | `pub extern "C" fn __block_app_sigs(old: *mut sigset_t);` | pthread_key_create |
| `__block_all_sigs` | `pub extern "C" fn __block_all_sigs(old: *mut sigset_t);` | pthread_kill |
| `__restore_sigs` | `pub extern "C" fn __restore_sigs(old: *mut sigset_t);` | pthread_key_create, pthread_kill |
| `__libc_sigaction` | `pub extern "C" fn __libc_sigaction(sig: c_int, act: *const c_void, oact: *mut c_void) -> c_int;` | pthread_cancel |

---

## 来自 `rusl_unistd` 的文件 I/O 接口

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `open` | `extern "C" fn open(path: *const c_char, flags: c_int, ...) -> c_int;` | sem_open, pthread_getname_np, pthread_setname_np |
| `close` | `extern "C" fn close(fd: c_int) -> c_int;` | sem_open, pthread_getname_np, pthread_setname_np |
| `read` | `extern "C" fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;` | pthread_getname_np |
| `write` | `extern "C" fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;` | sem_open, pthread_setname_np |
| `access` | `extern "C" fn access(path: *const c_char, mode: c_int) -> c_int;` | sem_open |
| `link` | `extern "C" fn link(old: *const c_char, new: *const c_char) -> c_int;` | sem_open |
| `unlink` | `extern "C" fn unlink(path: *const c_char) -> c_int;` | sem_open |
| `fstat` | `extern "C" fn fstat(fd: c_int, buf: *mut stat) -> c_int;` | sem_open |

---

## 来自 `rusl_syscall` 的内存映射接口

| C 接口 | Rust 调用方式 | 使用位置 |
|--------|-------------|----------|
| `mmap` | `rusl_syscall::syscall(SYS_mmap, ...) -> *mut c_void` | sem_open |
| `munmap` | `rusl_syscall::syscall(SYS_munmap, addr, len) -> c_int` | sem_open |
| `mremap` | `rusl_syscall::syscall(SYS_mremap, ...) -> *mut c_void` | pthread_getattr_np |
| `shm_unlink` | `pub extern "C" fn shm_unlink(name: *const c_char) -> c_int;` (来自 rusl_unistd) | sem_unlink |

---

## 来自 `rusl_time` 的时钟接口

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `clock_gettime` | `extern "C" fn clock_gettime(clk: clockid_t, ts: *mut timespec) -> c_int;` | sem_open |

---

## 来自 `rusl_string` 的字符串操作接口

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `strnlen` | `extern "C" fn strnlen(s: *const c_char, maxlen: usize) -> usize;` | pthread_setname_np |
| `memcmp` | `extern "C" fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;` | pthread_setattr_default_np |
| `memset` | `extern "C" fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;` | pthread_cancel |

---

## 来自 `rusl_stdio` 的格式化输出接口

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `snprintf` | `extern "C" fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;` | pthread_getname_np, pthread_setname_np, sem_open |

---

## 来自 `rusl_thread` 内部工具接口

| C 接口 | Rust extern "C" 签名 | 使用位置 |
|--------|---------------------|----------|
| `__shm_mapname` | `pub extern "C" fn __shm_mapname(name: *const c_char, buf: *mut c_char) -> *mut c_char;` | sem_open |

---

## 汇编/链接器符号

| C 符号 | Rust 声明 | 用途 | 使用位置 |
|--------|----------|------|---------|
| `__cp_begin` | `pub static __cp_begin: [u8; 0];` (链接器符号) | 取消点代码段起始标记 | pthread_cancel |
| `__cp_end` | `pub static __cp_end: [u8; 0];` (链接器符号) | 取消点代码段结束标记 | pthread_cancel |
| `__cp_cancel` | `pub static __cp_cancel: [u8; 0];` (链接器符号) | 取消跳转目标地址 | pthread_cancel |

Rust 中链接器符号通过 `extern "C" { static __cp_begin: [u8; 0]; }` 声明，取地址后转为 `*const c_void` 参与取消点范围的地址比较。

---

## 互斥锁字段访问常量

`pthread_mutex_t` 在 C 中为 union `{ __i: [c_int; ...], __vi: [c_int; ...], __p: [*mut c_void; ...] }`。
Rust 中以 `#[repr(C)]` union 表示，通过常量偏移或 `unsafe` 访问各字段。

| C 宏 | 访问字段 | Rust 等效 |
|------|----------|----------|
| `_m_type` | `__u.__i[0]` | `m.__u.__i[0]` 或 `(*m).type_field()` |
| `_m_lock` | `__u.__vi[1]` | `m.__u.__vi[1]` 或 `(*m).lock_field()` |
| `_m_waiters` | `__u.__vi[2]` | `m.__u.__vi[2]` 或 `(*m).waiters_field()` |
| `_m_prev` | `__u.__p[3]` | `m.__u.__p[3]` |
| `_m_next` | `__u.__p[4]` | `m.__u.__p[4]` |
| `_m_count` | `__u.__i[5]` | `m.__u.__i[5]` 或 `(*m).count_field()` |

---

## 属性成员访问常量

| C 符号 | 展开值 | Rust 等效 |
|--------|--------|----------|
| `__SU` | `sizeof(size_t) / sizeof(int)` | `pub const __SU: usize = size_of::<usize>() / size_of::<c_int>();` |
| `_a_stacksize` | `__u.__s[0]` | `attr.__u.__s[0]` |
| `_a_guardsize` | `__u.__s[1]` | `attr.__u.__s[1]` |
| `_a_stackaddr` | `__u.__s[2]` | `attr.__u.__s[2]` |
| `_a_detach` | `__u.__i[3*__SU+0]` | `attr.__u.__i[3 * __SU]` |
| `_a_sched` | `__u.__i[3*__SU+1]` | `attr.__u.__i[3 * __SU + 1]` |
| `_a_policy` | `__u.__i[3*__SU+2]` | `attr.__u.__i[3 * __SU + 2]` |
| `_a_prio` | `__u.__i[3*__SU+3]` | `attr.__u.__i[3 * __SU + 3]` |

---

## 全局变量

| C 符号 | Rust 声明 | 说明 | 使用位置 |
|--------|----------|------|---------|
| `__default_stacksize` | `pub static mut __default_stacksize: c_uint;` | 进程全局默认栈大小（初始值 131072） | pthread_attr_init, pthread_setattr_default_np |
| `__default_guardsize` | `pub static mut __default_guardsize: c_uint;` | 进程全局默认守护页大小（初始值 8192） | pthread_attr_init, pthread_setattr_default_np |
| `libc` | `pub static mut libc: __libc;` (来自 `rusl_internal`) | musl 全局运行时上下文 | pthread_getattr_np, __lock, __unlock |

---

## 默认值常量和宏

| C 宏 | Rust 定义 | 说明 | 使用位置 |
|------|----------|------|----------|
| `DEFAULT_STACK_SIZE` | `pub const DEFAULT_STACK_SIZE: usize = 131072;` (128KB) | default_attr |
| `DEFAULT_GUARD_SIZE` | `pub const DEFAULT_GUARD_SIZE: usize = 8192;` (8KB) | default_attr |
| `DEFAULT_STACK_MAX` | `pub const DEFAULT_STACK_MAX: usize = 8 << 20;` (8MB) | pthread_setattr_default_np |
| `DEFAULT_GUARD_MAX` | `pub const DEFAULT_GUARD_MAX: usize = 1 << 20;` (1MB) | pthread_setattr_default_np |
| `PAGE_SIZE` | `pub const PAGE_SIZE: usize = 4096;` (via `libc.page_size`) | pthread_getattr_np |
| `__ATTRP_C11_THREAD` | `pub const __ATTRP_C11_THREAD: *mut c_void = usize::MAX as *mut c_void;` | thrd_create |

---

## 标准头文件常量

| C 常量 | Rust 定义 | 说明 | 使用位置 |
|--------|----------|------|---------|
| `SIZE_MAX` | `pub const SIZE_MAX: usize = usize::MAX;` | size_t 最大值 | pthread_attr_setguardsize, pthread_attr_setstack, pthread_attr_setstacksize |
| `INT_MAX` | `pub const INT_MAX: c_int = i32::MAX;` | int 最大值 | sem_open, mutex trylock |
| `NAME_MAX` | `pub const NAME_MAX: c_int = 255;` | 最大文件名长度 | sem_open |
| `SIG_BLOCK` | `pub const SIG_BLOCK: c_int = 0;` | 阻塞信号 | pthread_sigmask |
| `SIG_UNBLOCK` | `pub const SIG_UNBLOCK: c_int = 1;` | 解阻塞信号 | pthread_sigmask |
| `SIG_SETMASK` | `pub const SIG_SETMASK: c_int = 2;` | 设置信号掩码 | pthread_sigmask |
| `SA_SIGINFO` | `pub const SA_SIGINFO: c_int = 4;` | sigaction 标志 | pthread_cancel |
| `SA_RESTART` | `pub const SA_RESTART: c_int = 0x10000000;` | sigaction 标志 | pthread_cancel |
| `SA_ONSTACK` | `pub const SA_ONSTACK: c_int = 0x08000000;` | sigaction 标志 | pthread_cancel |
| `_NSIG` | `pub const _NSIG: c_int = 65;` (x86_64) | 系统信号总数 | pthread_cancel, pthread_kill, pthread_sigmask |
| `CLOCK_REALTIME` | `pub const CLOCK_REALTIME: clockid_t = 0;` | 实时时钟 ID | sem_open, sem_timedwait, thrd_sleep |
| `O_RDONLY` | `pub const O_RDONLY: c_int = 0;` | 文件只读模式 | sem_open, pthread_getname_np |
| `O_WRONLY` | `pub const O_WRONLY: c_int = 1;` | 文件只写模式 | sem_open, pthread_setname_np |
| `O_RDWR` | `pub const O_RDWR: c_int = 2;` | 文件读写模式 | sem_open |
| `O_CREAT` | `pub const O_CREAT: c_int = 0o100;` | 创建文件标志 | sem_open |
| `O_EXCL` | `pub const O_EXCL: c_int = 0o200;` | 排他打开标志 | sem_open |
| `O_NOFOLLOW` | `pub const O_NOFOLLOW: c_int = 0o400000;` | 不跟随符号链接 | sem_open |
| `O_CLOEXEC` | `pub const O_CLOEXEC: c_int = 0o2000000;` | close-on-exec 标志 | sem_open, pthread_getname_np, pthread_setname_np |
| `O_NONBLOCK` | `pub const O_NONBLOCK: c_int = 0o4000;` | 非阻塞模式 | sem_open |
| `MAP_SHARED` | `pub const MAP_SHARED: c_int = 1;` | 共享映射 | sem_open |
| `MAP_FAILED` | `pub const MAP_FAILED: *mut c_void = usize::MAX as *mut c_void;` | mmap 失败返回值 | sem_open, pthread_getattr_np |
| `PROT_READ` | `pub const PROT_READ: c_int = 1;` | 可读内存保护 | sem_open |
| `PROT_WRITE` | `pub const PROT_WRITE: c_int = 2;` | 可写内存保护 | sem_open |
| `F_OK` | `pub const F_OK: c_int = 0;` | access() 存在性检查 | sem_open |
| `PR_SET_NAME` | `pub const PR_SET_NAME: c_int = 15;` | prctl 设置线程名 | pthread_setname_np |
| `PR_GET_NAME` | `pub const PR_GET_NAME: c_int = 16;` | prctl 获取线程名 | pthread_getname_np |

---

## 来自 `rusl_errno` 的错误码常量

| C 宏 | Rust 定义 | 说明 | 使用位置 |
|------|----------|------|---------|
| `EINVAL` | `pub const EINVAL: c_int = 22;` | 无效参数 | pthread_attr_set*, pthread_get/setconcurrency, pthread_kill, pthread_sigmask, pthread_setcancelstate/type, mutex settype/setpshared/setrobust/setprotocol/consistent/getprioceiling/setprioceiling, sem_init, sem_open |
| `ENOTSUP` | `pub const ENOTSUP: c_int = 95;` | 不支持操作 | pthread_attr_setscope, mutex setprotocol |
| `ENOMEM` | `pub const ENOMEM: c_int = 12;` | 内存不足 | pthread_getattr_np, pthread_atfork |
| `EBUSY` | `pub const EBUSY: c_int = 16;` | 自旋锁占用/互斥锁已被占用 | pthread_spin_lock, pthread_spin_trylock, mutex lock/trylock/timedlock, mtx_lock, mtx_trylock |
| `EAGAIN` | `pub const EAGAIN: c_int = 11;` | 资源暂时不可用 | pthread_setconcurrency, sem_trywait, pthread_key_create, mutex trylock |
| `EPERM` | `pub const EPERM: c_int = 1;` | 权限不足 | mutex consistent, mutex unlock |
| `EDEADLK` | `pub const EDEADLK: c_int = 35;` | 死锁检测 | mutex lock, mutex timedlock |
| `EOWNERDEAD` | `pub const EOWNERDEAD: c_int = 130;` | robust: 前一持有者已终止 | mutex trylock, mutex timedlock, mutex unlock |
| `ENOTRECOVERABLE` | `pub const ENOTRECOVERABLE: c_int = 131;` | robust: 不可恢复状态 | mutex trylock, mutex timedlock |
| `ETIMEDOUT` | `pub const ETIMEDOUT: c_int = 110;` | 操作超时 | sem_timedwait, mutex timedlock, thrd_sleep |
| `EINTR` | `pub const EINTR: c_int = 4;` | 系统调用被信号中断 | pthread_cancel, mutex timedlock, thrd_sleep |
| `ECANCELED` | `pub const ECANCELED: c_int = 125;` | 操作被取消 | pthread_cancel |
| `ENOSYS` | `pub const ENOSYS: c_int = 38;` | 系统调用不支持 | mutex setprotocol, setrobust, timedlock |
| `ERANGE` | `pub const ERANGE: c_int = 34;` | 结果超出范围 | pthread_getname_np, pthread_setname_np |
| `EOVERFLOW` | `pub const EOVERFLOW: c_int = 75;` | 值溢出 | sem_post |
| `EMFILE` | `pub const EMFILE: c_int = 24;` | 打开文件过多 | sem_open |
| `EEXIST` | `pub const EEXIST: c_int = 17;` | 文件已存在 | sem_open |
| `ENOENT` | `pub const ENOENT: c_int = 2;` | 文件未找到 | sem_open |

---

## 跨文件内部依赖

这些是 `rusl_thread` crate 内部不同 `.rs` 文件之间的调用关系，均为 `pub(crate)` 或 `pub extern "C"` 可见性。

### mutex 内部

| 被调函数 | 来源文件 | 调用者 |
|----------|---------|--------|
| `__pthread_mutex_trylock(m)` | `pthread_mutex_trylock.rs` | `pthread_mutex_lock.rs`, `pthread_mutex_timedlock.rs` |
| `__pthread_mutex_trylock_owner(m)` | `pthread_mutex_trylock.rs` | `pthread_mutex_trylock.rs` |
| `__pthread_mutex_timedlock(m, at)` | `pthread_mutex_timedlock.rs` | `pthread_mutex_lock.rs` |

### 取消相关

| 被调函数 | 来源文件 | 调用者 |
|----------|---------|--------|
| `pthread_self()` | `pthread_self.rs` | `pthread_cancel.rs` |
| `pthread_exit()` | `pthread_create.rs` | `pthread_cancel.rs` |
| `pthread_sigmask()` | `pthread_sigmask.rs` | `pthread_cancel.rs` |
| `pthread_kill()` | `pthread_kill.rs` | `pthread_cancel.rs` |
| `pthread_testcancel()` | `pthread_testcancel.rs` | `pthread_setcanceltype.rs` |
| `__testcancel()` | `pthread_cancel.rs` | `pthread_testcancel.rs` |
| `__cancel()` | `pthread_cancel.rs` | `pthread_testcancel.rs` (间接) |

---

## 编译器属性映射

| C 属性 | Rust 等效 |
|--------|----------|
| `weak_alias(old, new)` | `#[no_mangle] #[linkage = "weak"] pub extern "C" fn new(...) { ... }` 或在源码中直接调用 `old` 并通过单独的 `.rs` 文件定义弱别名 |
| `hidden` | `#[no_mangle]` (默认隐藏，使用 `#[export_name = "..."]` 或 `#[no_mangle]` 控制符号可见性) |

**注意**：Rust 的弱符号支持有限。对于 `weak_alias`，推荐在 Rust 中让别名函数直接委托给主实现（例如 `pthread_mutex_lock` 直接调用 `__pthread_mutex_lock`），而非依赖链接器弱符号机制。

---

## 依赖关系图

```
Level 0 (硬件/内核接口):
  core::sync::atomic (AtomicI32, Ordering)      // Rust 标准原子类型，替代 C 的 a_* 宏
  rusl_syscall (SYS_futex, SYS_tkill, SYS_rt_sigprocmask, ...) // 系统调用号 + syscall 封装
  TLS 寄存器读取 (x86_64: fs 段寄存器)           // __pthread_self 的底层实现

Level 1 (基础同步原语):
  __lock / __unlock                             // 自旋锁（基于 AtomicI32 + futex）
  __wait / __wake / __timedwait / __timedwait_cp // futex 等待/唤醒包装
  __vm_lock / __vm_unlock / __vm_wait           // VM 锁
  __tl_lock / __tl_unlock                       // 线程列表锁
  __acquire_ptc / __release_ptc / __inhibit_ptc // PTC 锁

Level 2 (线程基础设施):
  __pthread_self                                // 当前线程获取 (TLS)
  __pthread_create / __pthread_exit / __pthread_join // 线程生命周期
  __block_app_sigs / __block_all_sigs / __restore_sigs // 信号管理
  __libc_sigaction                              // 信号安装
  __do_cleanup_push / __do_cleanup_pop          // 清理栈
  __testcancel / __cancel                       // 取消机制

Level 3 (同步原语实现):
  __pthread_mutex_trylock / __pthread_mutex_timedlock / __pthread_mutex_unlock
  __pthread_rwlock_rdlock / __pthread_rwlock_wrlock / __pthread_rwlock_unlock
  __pthread_cond_timedwait / __private_cond_signal
  __pthread_once / __pthread_key_create / __pthread_key_delete

Level 4 (用户可见 API):
  pthread_mutex_lock / pthread_mutex_unlock / pthread_mutex_trylock / ...
  pthread_rwlock_rdlock / pthread_rwlock_wrlock / pthread_rwlock_unlock / ...
  pthread_cond_wait / pthread_cond_signal / pthread_cond_broadcast / ...
  pthread_create / pthread_join / pthread_detach / ...
  pthread_spin_lock / pthread_once / pthread_cancel / ...
  sem_wait / sem_post / sem_open / ...
  thrd_create / mtx_lock / cnd_wait / tss_set / call_once / ...
```

---

/* Rely */

[RELY]
Predefined Structures/Functions:
  上述所有来自 rusl_syscall、rusl_stdlib、rusl_unistd、rusl_string、rusl_stdio、
  rusl_internal、rusl_time、rusl_errno 的 extern "C" 导出接口
  rusl_thread crate 内部的 level 0-3 所有符号（__lock, __wait, __pthread_self, ...）
  Linux futex/tkill/prctl 系统调用

Predefined Macros/Crates:
  core::ffi            — Rust 核心库的 FFI 类型 (c_int, c_uint, c_char, c_void, c_long, size_t, isize)
  core::sync::atomic   — Rust 核心库的原子类型和操作 (AtomicI32, AtomicU32, Ordering)
  core::hint           — Rust 核心库的性能提示 (spin_loop)
  core::mem            — Rust 核心库的内存操作 (size_of, transmute)
  rusl_syscall         — rusl 系统调用封装模块
  rusl_stdlib          — rusl 内存分配接口 crate
  rusl_malloc          — rusl 低层内存分配器 crate
  rusl_unistd          — rusl 基本 I/O 和进程管理接口 crate
  rusl_string          — rusl 字符串和内存操作接口 crate
  rusl_stdio           — rusl 格式化 I/O 接口 crate
  rusl_internal        — rusl 内部辅助模块 (libc 全局上下文)
  rusl_time            — rusl 时间接口 crate
  rusl_errno           — rusl errno 错误码 crate
  rusl_thread          — 本 crate（线程基础设施）

[GUARANTEE]
Exported Interface:
  本文件仅记录依赖，不对外导出任何符号。

Internal Interface:
  rusl_thread crate 内部的 pub(crate) 和 pub extern "C" 符号构成了 level 0-3 的内部依赖图，
  确保 level 4 的用户可见 API 能够正确实现。
  所有需要跨模块 ABI 兼容的内部符号使用 `pub extern "C" fn` 导出，
  仅在 crate 内部使用的辅助函数使用 `pub(crate)` 可见性。
