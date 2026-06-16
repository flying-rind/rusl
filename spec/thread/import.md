# thread 模块 — 外部依赖接口

> 本文件记录 musl thread 模块所有源文件使用到的外部模块接口。

---

## 1. 原子操作（来自 `internal/atomic.h`）

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `a_cas` | `int a_cas(volatile int *p, int t, int s)` | 原子 compare-and-swap | pthread_spin_lock, pthread_spin_trylock, pthread_once, mutex lock/trylock/timedlock/unlock, sem_post, sem_trywait, sem_timedwait, mtx_lock, mtx_trylock |
| `a_swap` | `int a_swap(volatile int *p, int v)` | 原子交换 | pthread_once, mutex unlock |
| `a_store` | `void a_store(volatile int *p, int v)` | 原子存储（带内存屏障） | pthread_spin_unlock, pthread_cancel, mutex setprotocol/setrobust/timedlock/unlock |
| `a_spin` | `void a_spin(void)` | CPU 自旋/暂停提示 | pthread_spin_lock, sem_timedwait, mutex timedlock |
| `a_barrier` | `void a_barrier(void)` | 内存屏障 | pthread_once, pthread_cancel |
| `a_inc` | `void a_inc(volatile int *p)` | 原子递增 | sem_timedwait, mutex timedlock |
| `a_dec` | `void a_dec(volatile int *p)` | 原子递减 | sem_timedwait, mutex timedlock |
| `a_and` | `void a_and(volatile int *p, int v)` | 原子按位与 | mutex consistent |

---

## 2. Futex / 同步原语（来自 `pthread_impl.h`）

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `__wake` | `static inline void __wake(volatile void *addr, int cnt, int priv)` | futex 唤醒 | sem_post, pthread_once, mutex unlock |
| `__wait` | `hidden void __wait(volatile int *, volatile int *, int, int)` | futex 等待 | pthread_once |
| `__timedwait` | `hidden int __timedwait(volatile int *, int, clockid_t, const struct timespec *, int)` | futex 带超时阻塞等待 | mutex timedlock |
| `__timedwait_cp` | `hidden int __timedwait_cp(volatile int *, int, clockid_t, const struct timespec *, int)` | 可取消的 futex 等待 | sem_timedwait |

### Futex 常量

| 宏 | 值 | 说明 | 使用位置 |
|----|-----|------|---------|
| `FUTEX_WAIT` | 0 | 等待 futex | __futexwait |
| `FUTEX_WAKE` | 1 | 唤醒 futex 等待者 | __wake, mutex unlock |
| `FUTEX_LOCK_PI` | 6 | PI futex 加锁 | mutex timedlock, setprotocol |
| `FUTEX_UNLOCK_PI` | 7 | PI futex 解锁 | mutex timedlock, trylock, unlock |
| `FUTEX_PRIVATE` | 128 | 进程私有 futex 标志 | 所有同步原语 |
| `FUTEX_CLOCK_REALTIME` | 256 | 实时钟 futex | __futex4 |

---

## 3. 系统调用（来自 `internal/syscall.h`）

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `__syscall` | `long __syscall(long nr, ...)` | 原始系统调用 | pthread_cancel, pthread_kill, pthread_sigmask, mutex 所有文件 |
| `__syscall_cp_asm` | `hidden long __syscall_cp_asm(volatile void *, syscall_arg_t, ...)` | 取消点系统调用汇编版 | pthread_cancel |
| `__clock_nanosleep` | `int __clock_nanosleep(clockid_t, int, const struct timespec *, struct timespec *)` | 带时钟的睡眠 | thrd_sleep |

### 系统调用号

| 符号 | 值 | 说明 | 使用位置 |
|------|-----|------|---------|
| `SYS_futex` | 202 | futex 系统调用 | pthread_once, mutex |
| `SYS_futex_time64` | (可选) | 64 位时间 futex | mutex timedlock |
| `SYS_tkill` | 200 | 向线程发送信号 | pthread_cancel, pthread_kill |
| `SYS_close` | 3 | 关闭文件描述符 | pthread_cancel |
| `SYS_rt_sigprocmask` | 14 | 操作线程信号掩码 | pthread_sigmask |
| `SYS_sched_yield` | (架构相关) | 让出 CPU | thrd_yield |
| `SYS_get_robust_list` | (架构相关) | 探测内核 robust list 支持 | mutex setrobust |
| `SYS_set_robust_list` | (架构相关) | 注册线程 robust mutex 链表 | mutex trylock |

---

## 4. 自旋锁（来自 `internal/lock.h`）

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `LOCK(x)` / `__lock` | `void __lock(volatile int *)` | 获取自旋锁 | pthread_kill, pthread_atfork, sem_open |
| `UNLOCK(x)` / `__unlock` | `void __unlock(volatile int *)` | 释放自旋锁 | pthread_kill, pthread_atfork, sem_open |

---

## 5. 线程内部接口（来自 `pthread_impl.h`）

### 5.1 线程控制

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `__pthread_self` | `pthread_t __pthread_self(void)`（宏） | 获取当前线程 struct pthread * | pthread_cancel, pthread_once, pthread_setcancelstate/type, pthread_testcancel, pthread_self, mutex consistent/trylock/timedlock/unlock, pthread_key_create, pthread_getspecific, pthread_setspecific, tss_set |
| `__get_tp` | 架构特定内联/内建 | 读取 TLS 线程指针寄存器 | pthread_self (通过 __pthread_self 宏) |

### 5.2 线程生命周期

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `__pthread_create` | `int __pthread_create(thrd_t *, void *, void *(*)(void *), void *)` | 线程创建底层实现 | thrd_create |
| `__pthread_exit` | `void __pthread_exit(void *)` | 线程退出底层实现 | thrd_exit |
| `__pthread_join` | `int __pthread_join(thrd_t, void **)` | 线程等待底层实现 | thrd_join |

### 5.3 同步原语内部

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `__pthread_once` | `int __pthread_once(once_flag *, void (*)(void))` | once 底层实现 | call_once |
| `__pthread_mutex_timedlock` | `int __pthread_mutex_timedlock(pthread_mutex_t *, const struct timespec *)` | mutex 超时锁底层 | mtx_timedlock |
| `__pthread_mutex_trylock` | `int __pthread_mutex_trylock(pthread_mutex_t *)` | mutex 试探锁底层 | mtx_trylock |
| `__pthread_mutex_unlock` | `int __pthread_mutex_unlock(pthread_mutex_t *)` | mutex 解锁底层 | mtx_unlock |
| `__pthread_cond_timedwait` | `int __pthread_cond_timedwait(pthread_cond_t *, pthread_mutex_t *, const struct timespec *)` | cond 超时等待底层 | cnd_timedwait |
| `__private_cond_signal` | `int __private_cond_signal(pthread_cond_t *, int)` | cond 信号底层 | cnd_signal, cnd_broadcast |

### 5.4 TSD 内部

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `__pthread_key_create` | `int __pthread_key_create(tss_t *, void (*)(void *))` | key 创建底层 | tss_create |
| `__pthread_key_delete` | `void __pthread_key_delete(tss_t)` | key 删除底层 | tss_delete |

### 5.5 取消相关

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `__testcancel` | `hidden void __testcancel(void)` | 取消检查 | pthread_testcancel, pthread_cancel |
| `__do_cleanup_push` | `hidden void __do_cleanup_push(struct __ptcb *)` | 清理栈 push | pthread_cleanup_push |
| `__do_cleanup_pop` | `hidden void __do_cleanup_pop(struct __ptcb *)` | 清理栈 pop | pthread_cleanup_push |

### 5.6 PTC 锁

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `__acquire_ptc()` | hidden 函数 | 获取 PTC 读锁 | pthread_attr_init, pthread_setattr_default_np |
| `__release_ptc()` | hidden 函数 | 释放 PTC 锁 | pthread_attr_init, pthread_setattr_default_np |
| `__inhibit_ptc()` | hidden 函数 | 获取 PTC 写锁（阻止新线程创建） | pthread_setattr_default_np |

### 5.7 线程列表锁

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `__tl_lock` | `hidden void __tl_lock(void)` | 获取线程列表锁 | pthread_key_create |
| `__tl_unlock` | `hidden void __tl_unlock(void)` | 释放线程列表锁 | pthread_key_create |

### 5.8 VM 锁

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `__vm_lock()` | hidden 函数 | 获取虚拟内存区域锁 | mutex unlock |
| `__vm_unlock()` | hidden 函数 | 释放虚拟内存区域锁 | mutex unlock |
| `__vm_wait()` | hidden 函数 | 等待 VM 区域静默 | mutex destroy |

---

## 6. 内存管理

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `__libc_malloc` | `void *__libc_malloc(size_t)` | libc 内部 malloc（避免循环依赖） | pthread_atfork |
| `__libc_calloc` | `void *calloc(size_t nmemb, size_t size)` | 分配并零初始化内存 | sem_open |

---

## 7. 信号管理

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `__block_app_sigs` | `void __block_app_sigs(sigset_t *)` | 阻塞用户信号，保存旧信号集 | pthread_key_create |
| `__block_all_sigs` | `hidden void __block_all_sigs(sigset_t *)` | 阻塞全部信号 | pthread_kill |
| `__restore_sigs` | `hidden void __restore_sigs(sigset_t *)` | 恢复已保存的信号集 | pthread_key_create, pthread_kill |
| `__libc_sigaction` | `hidden int __libc_sigaction(int, const struct sigaction *, struct sigaction *)` | libc 内部 sigaction | pthread_cancel |

---

## 8. POSIX 文件 I/O（来自内核系统调用）

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `open` | `int open(const char *, int, ...)` | 打开文件 | sem_open, pthread_getname_np, pthread_setname_np |
| `close` | `int close(int)` | 关闭文件描述符 | sem_open, pthread_getname_np, pthread_setname_np |
| `read` | `ssize_t read(int, void *, size_t)` | 读取文件 | pthread_getname_np |
| `write` | `ssize_t write(int, const void *, size_t)` | 写入文件 | sem_open, pthread_setname_np |
| `access` | `int access(const char *, int)` | 检查文件访问权限 | sem_open |
| `link` | `int link(const char *, const char *)` | 创建硬链接 | sem_open |
| `unlink` | `int unlink(const char *)` | 删除文件名 | sem_open |
| `fstat` | `int fstat(int, struct stat *)` | 获取文件状态 | sem_open |

---

## 9. POSIX 内存映射（来自内核系统调用）

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `mmap` | `void *mmap(void *, size_t, int, int, int, off_t)` | 映射文件或匿名内存 | sem_open |
| `munmap` | `int munmap(void *, size_t)` | 解除内存映射 | sem_open |
| `mremap` | `void *mremap(void *, size_t, size_t, int)` | 内存区域重映射/调整大小 | pthread_getattr_np |
| `shm_unlink` | `int shm_unlink(const char *)` | 删除共享内存对象 | sem_unlink |

---

## 10. POSIX 时钟

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `clock_gettime` | `int clock_gettime(clockid_t, struct timespec *)` | 获取当前时间 | sem_open |

---

## 11. Linux 系统调用

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `prctl` | `int prctl(int, unsigned long, ...)` | 进程控制操作 | pthread_getname_np, pthread_setname_np |

---

## 12. 字符串操作（来自 `string` 模块）

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `strnlen` | `size_t strnlen(const char *, size_t)` | 带上限的字符串长度 | pthread_setname_np |
| `memcmp` | `int memcmp(const void *, const void *, size_t)` | 内存区域比较 | pthread_setattr_default_np |
| `memset` | `void *memset(void *, int, size_t)` | 内存设置 | pthread_cancel |

---

## 13. 格式化输出（来自 `stdio` 模块）

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `snprintf` | `int snprintf(char *, size_t, const char *, ...)` | 带边界检查的格式化字符串写入 | pthread_getname_np, pthread_setname_np, sem_open |

---

## 14. 内部工具

| 接口 | 签名 | 说明 | 使用位置 |
|------|------|------|---------|
| `__shm_mapname` | `hidden char *__shm_mapname(const char *, char *)` | 将 POSIX 名称转换为 `/dev/shm` 路径 | sem_open |

---

## 15. 汇编/链接器符号

| 符号 | 类型 | 用途 | 使用位置 |
|------|------|------|---------|
| `__cp_begin[]` | `extern hidden const char[]` | 取消点代码段起始标记 | pthread_cancel |
| `__cp_end[]` | `extern hidden const char[]` | 取消点代码段结束标记 | pthread_cancel |
| `__cp_cancel[]` | `extern hidden const char[]` | 取消跳转目标地址 | pthread_cancel |

---

## 16. 互斥锁字段访问宏（来自 `pthread_impl.h`）

| 宏 | 访问字段 | 说明 |
|----|----------|------|
| `_m_type` | `__u.__i[0]` | 互斥锁类型/属性位掩码 |
| `_m_lock` | `__u.__vi[1]` | 锁状态（owner tid / 标志位） |
| `_m_waiters` | `__u.__vi[2]` | 等待者计数/标志 |
| `_m_prev` | `__u.__p[3]` | robust_list 前驱指针 |
| `_m_next` | `__u.__p[4]` | robust_list 后继指针 |
| `_m_count` | `__u.__i[5]` | 递归计数（RECURSIVE 类型） |

---

## 17. 属性成员访问宏（来自 `pthread_impl.h`）

| 符号 | 展开值 | 说明 |
|------|--------|------|
| `__SU` | `sizeof(size_t) / sizeof(int)` | 跨 32/64 位平台兼容因子 |
| `_a_stacksize` | `__u.__s[0]` | 栈大小字段 |
| `_a_guardsize` | `__u.__s[1]` | 守护页大小字段 |
| `_a_stackaddr` | `__u.__s[2]` | 栈地址字段 |
| `_a_detach` | `__u.__i[3*__SU+0]` | 分离状态字段 |
| `_a_sched` | `__u.__i[3*__SU+1]` | 调度继承字段 |
| `_a_policy` | `__u.__i[3*__SU+2]` | 调度策略字段 |
| `_a_prio` | `__u.__i[3*__SU+3]` | 调度优先级字段 |

---

## 18. 线程内部结构体（来自 `pthread_impl.h`）

| 符号 | 类型 | 说明 | 使用位置 |
|------|------|------|---------|
| `struct pthread` | 结构体 | 线程控制块（含 tid, cancel, canceldisable, cancelasync, killlock, cancelbuf, detach_state, stack, stack_size, guard_size, robust_list, tsd, tsd_used 等字段） | 几乎所有文件 |
| `struct __ptcb` | 结构体 | 取消清理控制块（__f, __x, __next） | pthread_cleanup_push, pthread_once, mutex trylock/timedlock |
| `struct __libc` | 结构体 | musl 全局运行时上下文（含 auxv, page_size） | pthread_getattr_np |

---

## 19. 默认值常量和全局变量

| 符号 | 类型 | 值/说明 | 使用位置 |
|------|------|---------|----------|
| `__default_stacksize` | `unsigned` 全局变量 | 进程全局默认栈大小（初始值 131072） | pthread_attr_init, pthread_setattr_default_np |
| `__default_guardsize` | `unsigned` 全局变量 | 进程全局默认守护页大小（初始值 8192） | pthread_attr_init, pthread_setattr_default_np |
| `DEFAULT_STACK_SIZE` | 宏 | `131072` (128KB) | default_attr.c |
| `DEFAULT_GUARD_SIZE` | 宏 | `8192` (8KB) | default_attr.c |
| `DEFAULT_STACK_MAX` | 宏 | `8<<20` (8MB) | pthread_setattr_default_np |
| `DEFAULT_GUARD_MAX` | 宏 | `1<<20` (1MB) | pthread_setattr_default_np |
| `PAGE_SIZE` | 宏 | 展开为 `libc.page_size` | pthread_getattr_np |
| `__ATTRP_C11_THREAD` | 宏 | `((void*)(uintptr_t)-1)` | thrd_create |

---

## 20. C11 / POSIX 类型兼容性映射

| C11 类型 | POSIX 内部类型 | 关系 |
|----------|---------------|------|
| `cnd_t` | `pthread_cond_t` | 结构完全等同 |
| `mtx_t` | `pthread_mutex_t` | 结构完全等同（均含 `_m_type` 和 `_m_lock` 字段） |
| `thrd_t` | `struct __pthread *` | C 模式下为指向内部线程控制块的指针 |
| `once_flag` | `int` | 简单整数标志 |
| `tss_t` | `unsigned` | 用作 TSD 数组索引 |

---

## 21. 类型定义（来自标准头文件）

| 类型 | 来源 | 说明 |
|------|------|------|
| `pthread_t` | `<pthread.h>` / `bits/alltypes.h` | 线程标识符类型 |
| `pthread_attr_t` | `<pthread.h>` / `bits/alltypes.h` | 线程属性对象 |
| `pthread_mutex_t` | `<pthread.h>` / `bits/alltypes.h` | 互斥锁类型 |
| `pthread_mutexattr_t` | `<pthread.h>` / `bits/alltypes.h` | 互斥锁属性类型 |
| `pthread_rwlock_t` | `<pthread.h>` / `bits/alltypes.h` | 读写锁类型 |
| `pthread_rwlockattr_t` | `<pthread.h>` / `bits/alltypes.h` | 读写锁属性类型 |
| `pthread_cond_t` | `<pthread.h>` / `bits/alltypes.h` | 条件变量类型 |
| `pthread_condattr_t` | `<pthread.h>` / `bits/alltypes.h` | 条件变量属性类型 |
| `pthread_barrier_t` | `<pthread.h>` / `bits/alltypes.h` | 屏障类型 |
| `pthread_barrierattr_t` | `<pthread.h>` / `bits/alltypes.h` | 屏障属性类型 |
| `pthread_spinlock_t` | `<pthread.h>` / `bits/alltypes.h` | 自旋锁类型 |
| `pthread_once_t` | `<pthread.h>` / `bits/alltypes.h` | 一次性初始化控制变量 |
| `pthread_key_t` | `<pthread.h>` / `bits/alltypes.h` | TSD 键类型 |
| `sem_t` | `<semaphore.h>` | 信号量类型 |
| `sigset_t` | `<signal.h>` / `bits/alltypes.h` | 信号集类型 |
| `struct timespec` | `<time.h>` / `bits/alltypes.h` | 时间规格：`{ time_t tv_sec; long tv_nsec; }` |
| `struct sched_param` | `<sched.h>` | 调度参数结构体 |
| `struct stat` | `<sys/stat.h>` | 文件状态结构体 |
| `mode_t` | `<sys/stat.h>` / `bits/alltypes.h` | 文件模式类型 |
| `ino_t` | `<sys/stat.h>` / `bits/alltypes.h` | inode 号类型 |
| `va_list` | `<stdarg.h>` | 可变参数列表类型 |
| `size_t` | `<stddef.h>` | 无符号大小类型 |
| `intptr_t` | `<stdint.h>` | 整数-指针互转类型 |
| `syscall_arg_t` | `pthread_impl.h` | 系统调用参数类型 (`long`) |

---

## 22. 错误码常量（来自 `<errno.h>`）

| 宏 | 值 | 说明 | 使用位置 |
|----|-----|------|---------|
| `EINVAL` | 22 | 无效参数 | pthread_attr_set*, pthread_get/setconcurrency, pthread_kill, pthread_sigmask, pthread_setcancelstate/type, mutex settype/setpshared/setrobust/setprotocol/consistent/getprioceiling/setprioceiling, sem_init, sem_open |
| `ENOTSUP` | 96 | 不支持操作 | pthread_attr_setscope, mutex setprotocol |
| `ENOMEM` | 12 | 内存不足 | pthread_getattr_np, pthread_atfork |
| `EBUSY` | 16 | 自旋锁占用/互斥锁已被占用 | pthread_spin_lock, pthread_spin_trylock, mutex lock/trylock/timedlock, mtx_lock, mtx_trylock |
| `EAGAIN` | 11 | 资源暂时不可用 | pthread_setconcurrency, sem_trywait, pthread_key_create, mutex trylock |
| `EPERM` | 1 | 权限不足 | mutex consistent, mutex unlock |
| `EDEADLK` | 35 | 死锁检测 | mutex lock, mutex timedlock |
| `EOWNERDEAD` | 130 | robust: 前一持有者已终止 | mutex trylock, mutex timedlock, mutex unlock |
| `ENOTRECOVERABLE` | 131 | robust: 不可恢复状态 | mutex trylock, mutex timedlock |
| `ETIMEDOUT` | 110 | 操作超时 | sem_timedwait, mutex timedlock, thrd_sleep |
| `EINTR` | 4 | 系统调用被信号中断 | pthread_cancel, mutex timedlock, thrd_sleep |
| `ECANCELED` | 125 | 操作被取消 | pthread_cancel |
| `ENOSYS` | 38 | 系统调用不支持 | mutex setprotocol, setrobust, timedlock |
| `ERANGE` | 34 | 结果超出范围 | pthread_getname_np, pthread_setname_np |
| `EOVERFLOW` | 75 | 值溢出 | sem_post |
| `EMFILE` | 24 | 打开文件过多 | sem_open |
| `EEXIST` | 17 | 文件已存在 | sem_open |
| `ENOENT` | 2 | 文件未找到 | sem_open |

---

## 23. 标准头文件常量

| 常量 | 值 | 说明 | 使用位置 |
|------|-----|------|---------|
| `SIZE_MAX` | (平台相关) | size_t 类型最大值 | pthread_attr_setguardsize, pthread_attr_setstack, pthread_attr_setstacksize |
| `INT_MAX` | 0x7FFFFFFF | int 类型最大值 | sem_open, mutex trylock |
| `NAME_MAX` | 255 | 最大文件名长度 | sem_open |
| `SIG_BLOCK` / `SIG_UNBLOCK` / `SIG_SETMASK` | 0/1/2 | 信号掩码操作 | pthread_sigmask |
| `SA_SIGINFO` / `SA_RESTART` / `SA_ONSTACK` | (平台相关) | sigaction 标志 | pthread_cancel |
| `_NSIG` | (平台相关) | 系统信号总数 | pthread_cancel, pthread_kill, pthread_sigmask |
| `CLOCK_REALTIME` | 0 | 实时时钟 ID | sem_open, sem_timedwait, thrd_sleep |
| `O_RDONLY` / `O_WRONLY` / `O_RDWR` | 0/1/2 | 文件打开模式 | sem_open, pthread_getname_np, pthread_setname_np |
| `O_CREAT` / `O_EXCL` / `O_NOFOLLOW` / `O_CLOEXEC` / `O_NONBLOCK` | (平台相关) | 文件打开标志 | sem_open, pthread_getname_np, pthread_setname_np |
| `MAP_SHARED` / `MAP_FAILED` / `PROT_READ` / `PROT_WRITE` | 1/-1/1/2 | mmap 标志 | sem_open, pthread_getattr_np |
| `F_OK` | 0 | access() 存在性检查 | sem_open |
| `PR_SET_NAME` / `PR_GET_NAME` | 15/16 | prctl 线程名称操作 | pthread_getname_np, pthread_setname_np |

---

## 24. 编译器属性

| 接口 | 说明 |
|------|------|
| `weak_alias(old, new)` | 弱符号别名宏。展开为 `extern __typeof(old) new __attribute__((__weak__, __alias__(#old)))` |
| `hidden` | 符号可见性属性，限制符号在 DSO 内部可见 |

---

## 25. 跨文件内部依赖

### 25.1 mutex 内部

| 被调函数 | 来源 | 调用者 |
|----------|------|--------|
| `__pthread_mutex_trylock(m)` | pthread_mutex_trylock.c | lock, timedlock, timedlock_pi |
| `__pthread_mutex_trylock_owner(m)` | pthread_mutex_trylock.c | trylock |
| `__pthread_mutex_timedlock(m, at)` | pthread_mutex_timedlock.c | lock |

### 25.2 取消相关

| 被调函数 | 来源 | 调用者 |
|----------|------|--------|
| `pthread_self()` | pthread_self.c | pthread_cancel |
| `pthread_exit()` | pthread_create.c | pthread_cancel |
| `pthread_sigmask()` | pthread_sigmask.c | pthread_cancel |
| `pthread_kill()` | pthread_kill.c | pthread_cancel |
| `pthread_testcancel()` | pthread_testcancel.c | pthread_setcanceltype |
| `__testcancel()` | pthread_cancel.c | pthread_testcancel |
| `__cancel()` | pthread_cancel.c | pthread_testcancel（间接） |

---

## 26. 依赖关系图

```
Level 0 (硬件/内核接口):
  atomic.h (a_cas, a_swap, a_store, a_spin, a_barrier, a_inc, a_dec, a_and)
  syscall.h (__syscall, __syscall_cp_asm, SYS_futex, SYS_tkill, SYS_rt_sigprocmask, ...)
  __get_tp (TLS 寄存器读取)

Level 1 (基础同步原语):
  __lock / __unlock (自旋锁)
  __wait / __wake / __timedwait (futex 包装)
  __vm_lock / __vm_unlock / __vm_wait (VM 锁)
  __tl_lock / __tl_unlock (线程列表锁)
  __acquire_ptc / __release_ptc / __inhibit_ptc (PTC 锁)

Level 2 (线程基础设施):
  __pthread_self (当前线程获取)
  __pthread_create / __pthread_exit / __pthread_join (线程生命周期)
  __block_app_sigs / __block_all_sigs / __restore_sigs (信号管理)
  __libc_sigaction
  __do_cleanup_push / __do_cleanup_pop (清理栈)
  __testcancel / __cancel (取消机制)

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
