# thread 模块 — 对外导出接口

> 本文件记录 musl thread 模块所有对外导出的 API。接口通过 `<pthread.h>`、`<threads.h>` 或 `<semaphore.h>` 声明，用户程序可直接调用。

---

## 1. 线程属性 (pthread_attr) API

### 1.1 属性初始化与销毁

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_attr_init(pthread_attr_t *a)` | 初始化线程属性对象为默认值 | pthread_attr_init.c |
| `int pthread_attr_destroy(pthread_attr_t *a)` | 销毁线程属性对象（空操作） | pthread_attr_destroy.c |

### 1.2 属性获取 (Getter)

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_attr_getdetachstate(const pthread_attr_t *a, int *state)` | 获取分离状态 | pthread_attr_get.c |
| `int pthread_attr_getguardsize(const pthread_attr_t *restrict a, size_t *restrict size)` | 获取守护页大小 | pthread_attr_get.c |
| `int pthread_attr_getinheritsched(const pthread_attr_t *restrict a, int *restrict inherit)` | 获取调度继承策略 | pthread_attr_get.c |
| `int pthread_attr_getschedparam(const pthread_attr_t *restrict a, struct sched_param *restrict param)` | 获取调度参数 | pthread_attr_get.c |
| `int pthread_attr_getschedpolicy(const pthread_attr_t *restrict a, int *restrict policy)` | 获取调度策略 | pthread_attr_get.c |
| `int pthread_attr_getscope(const pthread_attr_t *restrict a, int *restrict scope)` | 获取竞争范围 | pthread_attr_get.c |
| `int pthread_attr_getstack(const pthread_attr_t *restrict a, void **restrict addr, size_t *restrict size)` | 获取栈地址和大小 | pthread_attr_get.c |
| `int pthread_attr_getstacksize(const pthread_attr_t *restrict a, size_t *restrict size)` | 获取栈大小 | pthread_attr_get.c |

### 1.3 属性设置 (Setter)

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_attr_setdetachstate(pthread_attr_t *a, int state)` | 设置分离状态 | pthread_attr_setdetachstate.c |
| `int pthread_attr_setguardsize(pthread_attr_t *a, size_t size)` | 设置守护页大小 | pthread_attr_setguardsize.c |
| `int pthread_attr_setinheritsched(pthread_attr_t *a, int inherit)` | 设置调度继承策略 | pthread_attr_setinheritsched.c |
| `int pthread_attr_setschedparam(pthread_attr_t *restrict a, const struct sched_param *restrict param)` | 设置调度参数 | pthread_attr_setschedparam.c |
| `int pthread_attr_setschedpolicy(pthread_attr_t *a, int policy)` | 设置调度策略 | pthread_attr_setschedpolicy.c |
| `int pthread_attr_setscope(pthread_attr_t *a, int scope)` | 设置竞争范围 | pthread_attr_setscope.c |
| `int pthread_attr_setstack(pthread_attr_t *a, void *addr, size_t size)` | 设置栈地址和大小 | pthread_attr_setstack.c |
| `int pthread_attr_setstacksize(pthread_attr_t *a, size_t size)` | 设置栈大小 | pthread_attr_setstacksize.c |

### 1.4 其他属性类型获取

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_barrierattr_getpshared(const pthread_barrierattr_t *restrict a, int *restrict pshared)` | 获取 barrier 进程共享标志 | pthread_attr_get.c |
| `int pthread_condattr_getclock(const pthread_condattr_t *restrict a, clockid_t *restrict clk)` | 获取条件变量时钟 | pthread_attr_get.c |
| `int pthread_condattr_getpshared(const pthread_condattr_t *restrict a, int *restrict pshared)` | 获取条件变量进程共享标志 | pthread_attr_get.c |
| `int pthread_mutexattr_getprotocol(const pthread_mutexattr_t *restrict a, int *restrict protocol)` | 获取互斥锁优先级协议 | pthread_attr_get.c |
| `int pthread_mutexattr_getpshared(const pthread_mutexattr_t *restrict a, int *restrict pshared)` | 获取互斥锁进程共享标志 | pthread_attr_get.c |
| `int pthread_mutexattr_getrobust(const pthread_mutexattr_t *restrict a, int *restrict robust)` | 获取互斥锁健壮性 | pthread_attr_get.c |
| `int pthread_mutexattr_gettype(const pthread_mutexattr_t *restrict a, int *restrict type)` | 获取互斥锁类型 | pthread_attr_get.c |
| `int pthread_rwlockattr_getpshared(const pthread_rwlockattr_t *restrict a, int *restrict pshared)` | 获取读写锁进程共享标志 | pthread_attr_get.c |

### 1.5 GNU 扩展（需 `_GNU_SOURCE`）

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_getattr_default_np(pthread_attr_t *attrp)` | 获取进程全局默认线程属性 | pthread_setattr_default_np.c |
| `int pthread_setattr_default_np(const pthread_attr_t *attrp)` | 设置进程全局默认线程属性 | pthread_setattr_default_np.c |
| `int pthread_getattr_np(pthread_t t, pthread_attr_t *a)` | 从已存在线程获取实际属性 | pthread_getattr_np.c |

---

## 2. 互斥锁 (pthread_mutex) API

### 2.1 互斥锁属性操作

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_mutexattr_init(pthread_mutexattr_t *a)` | 初始化互斥锁属性对象 | pthread_mutexattr_init.c |
| `int pthread_mutexattr_destroy(pthread_mutexattr_t *a)` | 销毁互斥锁属性对象（空操作） | pthread_mutexattr_destroy.c |
| `int pthread_mutexattr_settype(pthread_mutexattr_t *a, int type)` | 设置互斥锁类型 | pthread_mutexattr_settype.c |
| `int pthread_mutexattr_setpshared(pthread_mutexattr_t *a, int pshared)` | 设置进程共享标志 | pthread_mutexattr_setpshared.c |
| `int pthread_mutexattr_setrobust(pthread_mutexattr_t *a, int robust)` | 设置健壮性标志 | pthread_mutexattr_setrobust.c |
| `int pthread_mutexattr_setprotocol(pthread_mutexattr_t *a, int protocol)` | 设置优先级协议 | pthread_mutexattr_setprotocol.c |

### 2.2 互斥锁生命周期

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_mutex_init(pthread_mutex_t *restrict m, const pthread_mutexattr_t *restrict a)` | 初始化互斥锁 | pthread_mutex_init.c |
| `int pthread_mutex_destroy(pthread_mutex_t *mutex)` | 销毁互斥锁 | pthread_mutex_destroy.c |

### 2.3 互斥锁同步操作

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_mutex_lock(pthread_mutex_t *m)` | 阻塞加锁 | pthread_mutex_lock.c |
| `int pthread_mutex_trylock(pthread_mutex_t *m)` | 非阻塞尝试加锁 | pthread_mutex_trylock.c |
| `int pthread_mutex_timedlock(pthread_mutex_t *restrict m, const struct timespec *restrict at)` | 带超时加锁 | pthread_mutex_timedlock.c |
| `int pthread_mutex_unlock(pthread_mutex_t *m)` | 解锁 | pthread_mutex_unlock.c |

### 2.4 健壮互斥锁与优先级

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_mutex_consistent(pthread_mutex_t *m)` | 将 EOWNERDEAD 互斥锁标记为一致 | pthread_mutex_consistent.c |
| `int pthread_mutex_getprioceiling(const pthread_mutex_t *restrict m, int *restrict ceiling)` | 获取优先级天花板（始终返回 EINVAL） | pthread_mutex_getprioceiling.c |
| `int pthread_mutex_setprioceiling(pthread_mutex_t *restrict m, int ceiling, int *restrict old)` | 设置优先级天花板并加锁（始终返回 EINVAL） | pthread_mutex_setprioceiling.c |

---

## 3. 读写锁 (pthread_rwlock) API

### 3.1 读写锁属性

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_rwlockattr_init(pthread_rwlockattr_t *a)` | 初始化读写锁属性对象 | pthread_rwlockattr_init.c |
| `int pthread_rwlockattr_destroy(pthread_rwlockattr_t *a)` | 销毁读写锁属性对象 | pthread_rwlockattr_destroy.c |
| `int pthread_rwlockattr_setpshared(pthread_rwlockattr_t *a, int pshared)` | 设置进程共享属性 | pthread_rwlockattr_setpshared.c |

### 3.2 读写锁操作

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_rwlock_init(pthread_rwlock_t *restrict rw, const pthread_rwlockattr_t *restrict a)` | 初始化读写锁 | pthread_rwlock_init.c |
| `int pthread_rwlock_destroy(pthread_rwlock_t *rw)` | 销毁读写锁 | pthread_rwlock_destroy.c |
| `int pthread_rwlock_rdlock(pthread_rwlock_t *rw)` | 阻塞获取读锁 | pthread_rwlock_rdlock.c |
| `int pthread_rwlock_tryrdlock(pthread_rwlock_t *rw)` | 非阻塞尝试获取读锁 | pthread_rwlock_tryrdlock.c |
| `int pthread_rwlock_timedrdlock(pthread_rwlock_t *restrict rw, const struct timespec *restrict at)` | 带超时获取读锁 | pthread_rwlock_timedrdlock.c |
| `int pthread_rwlock_wrlock(pthread_rwlock_t *rw)` | 阻塞获取写锁 | pthread_rwlock_wrlock.c |
| `int pthread_rwlock_trywrlock(pthread_rwlock_t *rw)` | 非阻塞尝试获取写锁 | pthread_rwlock_trywrlock.c |
| `int pthread_rwlock_timedwrlock(pthread_rwlock_t *restrict rw, const struct timespec *restrict at)` | 带超时获取写锁 | pthread_rwlock_timedwrlock.c |
| `int pthread_rwlock_unlock(pthread_rwlock_t *rw)` | 释放读锁或写锁 | pthread_rwlock_unlock.c |

---

## 4. 条件变量 (pthread_cond) API

### 4.1 条件变量属性

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_condattr_init(pthread_condattr_t *a)` | 初始化条件变量属性 | pthread_condattr_init.c |
| `int pthread_condattr_destroy(pthread_condattr_t *a)` | 销毁条件变量属性 | pthread_condattr_destroy.c |
| `int pthread_condattr_setclock(pthread_condattr_t *a, clockid_t clk)` | 设置等待时钟 | pthread_condattr_setclock.c |
| `int pthread_condattr_setpshared(pthread_condattr_t *a, int pshared)` | 设置进程共享标志 | pthread_condattr_setpshared.c |

### 4.2 条件变量操作

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_cond_init(pthread_cond_t *restrict c, const pthread_condattr_t *restrict a)` | 初始化条件变量 | pthread_cond_init.c |
| `int pthread_cond_destroy(pthread_cond_t *c)` | 销毁条件变量 | pthread_cond_destroy.c |
| `int pthread_cond_wait(pthread_cond_t *restrict c, pthread_mutex_t *restrict m)` | 无限等待条件变量 | pthread_cond_wait.c |
| `int pthread_cond_timedwait(pthread_cond_t *restrict c, pthread_mutex_t *restrict m, const struct timespec *restrict ts)` | 带超时等待条件变量 | pthread_cond_timedwait.c |
| `int pthread_cond_signal(pthread_cond_t *c)` | 唤醒一个等待线程 | pthread_cond_signal.c |
| `int pthread_cond_broadcast(pthread_cond_t *c)` | 唤醒所有等待线程 | pthread_cond_broadcast.c |

---

## 5. 屏障 (pthread_barrier) API

### 5.1 屏障属性

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_barrierattr_init(pthread_barrierattr_t *a)` | 初始化屏障属性 | pthread_barrierattr_init.c |
| `int pthread_barrierattr_destroy(pthread_barrierattr_t *a)` | 销毁屏障属性 | pthread_barrierattr_destroy.c |
| `int pthread_barrierattr_setpshared(pthread_barrierattr_t *a, int pshared)` | 设置进程共享标志 | pthread_barrierattr_setpshared.c |

### 5.2 屏障操作

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_barrier_init(pthread_barrier_t *restrict b, const pthread_barrierattr_t *restrict a, unsigned count)` | 初始化屏障 | pthread_barrier_init.c |
| `int pthread_barrier_destroy(pthread_barrier_t *b)` | 销毁屏障 | pthread_barrier_destroy.c |
| `int pthread_barrier_wait(pthread_barrier_t *b)` | 在屏障上等待 | pthread_barrier_wait.c |

---

## 6. 自旋锁 (pthread_spin) API

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_spin_init(pthread_spinlock_t *s, int pshared)` | 初始化自旋锁为解锁状态 | pthread_spin_init.c |
| `int pthread_spin_destroy(pthread_spinlock_t *s)` | 销毁自旋锁（空操作） | pthread_spin_destroy.c |
| `int pthread_spin_lock(pthread_spinlock_t *s)` | 忙等待获取自旋锁 | pthread_spin_lock.c |
| `int pthread_spin_trylock(pthread_spinlock_t *s)` | 非阻塞尝试获取自旋锁 | pthread_spin_trylock.c |
| `int pthread_spin_unlock(pthread_spinlock_t *s)` | 释放自旋锁 | pthread_spin_unlock.c |

---

## 7. 一次性初始化 (pthread_once) API

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_once(pthread_once_t *control, void (*init)(void))` | 多线程安全的一次性初始化 | pthread_once.c |

---

## 8. 线程生命周期 API

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_create(pthread_t *restrict t, const pthread_attr_t *restrict a, void *(*f)(void *), void *restrict arg)` | 创建新线程 | pthread_create.c |
| `void pthread_exit(void *retval)` | 终止当前线程 | pthread_create.c |
| `int pthread_join(pthread_t t, void **res)` | 等待线程终止并获取返回值 | pthread_join.c |
| `int pthread_detach(pthread_t t)` | 分离线程（回收资源不等待） | pthread_detach.c |

---

## 9. 线程取消 API

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_cancel(pthread_t t)` | 向目标线程发送取消请求 | pthread_cancel.c |
| `void pthread_testcancel(void)` | 显式取消点 | pthread_testcancel.c |
| `int pthread_setcancelstate(int state, int *oldstate)` | 设置线程取消启用/禁用状态 | pthread_setcancelstate.c |
| `int pthread_setcanceltype(int type, int *oldtype)` | 设置线程取消类型（延迟/异步） | pthread_setcanceltype.c |

---

## 10. 线程调度 API

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_getschedparam(pthread_t t, int *restrict policy, struct sched_param *restrict param)` | 获取线程调度策略和优先级 | pthread_getschedparam.c |
| `int pthread_setschedparam(pthread_t t, int policy, const struct sched_param *param)` | 设置线程调度策略和优先级 | pthread_setschedparam.c |
| `int pthread_setschedprio(pthread_t t, int prio)` | 设置线程优先级 | pthread_setschedprio.c |
| `int pthread_getconcurrency(void)` | 获取并发级别（废弃，始终返回 0） | pthread_getconcurrency.c |
| `int pthread_setconcurrency(int val)` | 设置并发级别（废弃） | pthread_setconcurrency.c |
| `int pthread_getcpuclockid(pthread_t t, clockid_t *clk)` | 获取线程 CPU 时钟 ID | pthread_getcpuclockid.c |

---

## 11. 线程标识 API

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `pthread_t pthread_self(void)` | 获取当前线程标识符 | pthread_self.c |
| `int pthread_equal(pthread_t a, pthread_t b)` | 比较两个线程标识符 | pthread_equal.c |

---

## 12. 线程信号 API

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_kill(pthread_t t, int sig)` | 向指定线程发送信号 | pthread_kill.c |
| `int pthread_sigmask(int how, const sigset_t *restrict set, sigset_t *restrict old)` | 检查/修改线程信号掩码 | pthread_sigmask.c |

---

## 13. 清理处理 API

| 符号 | 形式 | 简短说明 | 源文件 |
|------|------|---------|--------|
| `pthread_cleanup_push` | 宏: `pthread_cleanup_push(f, x)` | 压入取消清理栈 | pthread_cleanup_push.c |
| `pthread_cleanup_pop` | 宏: `pthread_cleanup_pop(r)` | 弹出取消清理栈 | pthread_cleanup_push.c |
| `_pthread_cleanup_push` | `void _pthread_cleanup_push(struct __ptcb *, void (*)(void *), void *)` | 清理栈 push 底层函数 | pthread_cleanup_push.c |
| `_pthread_cleanup_pop` | `void _pthread_cleanup_pop(struct __ptcb *, int)` | 清理栈 pop 底层函数 | pthread_cleanup_push.c |

---

## 14. Fork 处理 API

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_atfork(void (*prepare)(void), void (*parent)(void), void (*child)(void))` | 注册 fork 前/后回调 | pthread_atfork.c |

---

## 15. 线程局部存储 (TSD) API

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_key_create(pthread_key_t *k, void (*dtor)(void *))` | 创建 TSD 键 | pthread_key_create.c |
| `int pthread_key_delete(pthread_key_t k)` | 删除 TSD 键 | pthread_key_create.c |
| `void *pthread_getspecific(pthread_key_t k)` | 获取当前线程的 TSD 值 | pthread_getspecific.c |
| `int pthread_setspecific(pthread_key_t k, const void *x)` | 设置当前线程的 TSD 值 | pthread_setspecific.c |

---

## 16. 线程命名 API（GNU 扩展）

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int pthread_setname_np(pthread_t thread, const char *name)` | 设置线程名称（最多 15 字符） | pthread_setname_np.c |
| `int pthread_getname_np(pthread_t thread, char *name, size_t len)` | 获取线程名称 | pthread_getname_np.c |

---

## 17. 信号量 API

### 17.1 匿名信号量

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int sem_init(sem_t *sem, int pshared, unsigned value)` | 初始化匿名信号量 | sem_init.c |
| `int sem_destroy(sem_t *sem)` | 销毁匿名信号量 | sem_destroy.c |
| `int sem_getvalue(sem_t *restrict sem, int *restrict valp)` | 查询信号量当前值 | sem_getvalue.c |
| `int sem_wait(sem_t *sem)` | 阻塞递减（P 操作） | sem_wait.c |
| `int sem_trywait(sem_t *sem)` | 非阻塞试探递减 | sem_trywait.c |
| `int sem_timedwait(sem_t *restrict sem, const struct timespec *restrict at)` | 带超时的阻塞递减 | sem_timedwait.c |
| `int sem_post(sem_t *sem)` | 递增（V 操作） | sem_post.c |

### 17.2 有名信号量

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `sem_t *sem_open(const char *name, int flags, ...)` | 打开/创建有名信号量 | sem_open.c |
| `int sem_close(sem_t *sem)` | 关闭有名信号量 | sem_open.c |
| `int sem_unlink(const char *name)` | 从系统移除有名信号量 | sem_unlink.c |

---

## 18. C11 线程 (thrd/cnd/mtx/tss/call_once) API

声明于 `<threads.h>`。

### 18.1 一次性执行

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `void call_once(once_flag *flag, void (*func)(void))` | 确保 func 恰好执行一次 | call_once.c |

### 18.2 条件变量

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int cnd_init(cnd_t *c)` | 初始化条件变量 | cnd_init.c |
| `void cnd_destroy(cnd_t *c)` | 销毁条件变量 | cnd_destroy.c |
| `int cnd_wait(cnd_t *c, mtx_t *m)` | 在条件变量上等待（无限期） | cnd_wait.c |
| `int cnd_timedwait(cnd_t *restrict c, mtx_t *restrict m, const struct timespec *restrict ts)` | 在条件变量上等待（带超时） | cnd_timedwait.c |
| `int cnd_signal(cnd_t *c)` | 唤醒一个等待线程 | cnd_signal.c |
| `int cnd_broadcast(cnd_t *c)` | 唤醒所有等待线程 | cnd_broadcast.c |

### 18.3 互斥锁

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int mtx_init(mtx_t *m, int type)` | 初始化互斥锁 | mtx_init.c |
| `void mtx_destroy(mtx_t *mtx)` | 销毁互斥锁 | mtx_destroy.c |
| `int mtx_lock(mtx_t *m)` | 锁定互斥锁（阻塞） | mtx_lock.c |
| `int mtx_timedlock(mtx_t *restrict m, const struct timespec *restrict ts)` | 锁定互斥锁（带超时） | mtx_timedlock.c |
| `int mtx_trylock(mtx_t *m)` | 尝试锁定互斥锁（非阻塞） | mtx_trylock.c |
| `int mtx_unlock(mtx_t *mtx)` | 解锁互斥锁 | mtx_unlock.c |

### 18.4 线程管理

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int thrd_create(thrd_t *thr, thrd_start_t func, void *arg)` | 创建新线程 | thrd_create.c |
| `_Noreturn void thrd_exit(int result)` | 终止当前线程 | thrd_exit.c |
| `int thrd_join(thrd_t t, int *res)` | 等待线程终止并获取退出码 | thrd_join.c |
| `int thrd_sleep(const struct timespec *req, struct timespec *rem)` | 线程睡眠指定时长 | thrd_sleep.c |
| `void thrd_yield(void)` | 让出 CPU | thrd_yield.c |
| `pthread_t thrd_current(void)` | C11 版 pthread_self | pthread_self.c |
| `int thrd_equal(pthread_t, pthread_t)` | C11 版 pthread_equal | pthread_equal.c |

### 18.5 线程特定存储

| 函数签名 | 简短说明 | 源文件 |
|----------|---------|--------|
| `int tss_create(tss_t *tss, tss_dtor_t dtor)` | 创建 TSS 键 | tss_create.c |
| `void tss_delete(tss_t key)` | 删除 TSS 键 | tss_delete.c |
| `int tss_set(tss_t k, void *x)` | 设置当前线程的 TSS 值 | tss_set.c |
| `void *tss_get(tss_t k)` | C11 版 pthread_getspecific（弱别名） | pthread_getspecific.c |

---

## 19. 内部弱别名映射

musl 内部使用 `__` 前缀的符号作为主实现，无前缀的 POSIX 名称通过 `weak_alias` 映射：

### 19.1 rwlock

| 内部符号 | 对外导出符号 | 源文件 |
|----------|-------------|--------|
| `__pthread_rwlock_rdlock` | `pthread_rwlock_rdlock` | pthread_rwlock_rdlock.c |
| `__pthread_rwlock_tryrdlock` | `pthread_rwlock_tryrdlock` | pthread_rwlock_tryrdlock.c |
| `__pthread_rwlock_timedrdlock` | `pthread_rwlock_timedrdlock` | pthread_rwlock_timedrdlock.c |
| `__pthread_rwlock_wrlock` | `pthread_rwlock_wrlock` | pthread_rwlock_wrlock.c |
| `__pthread_rwlock_trywrlock` | `pthread_rwlock_trywrlock` | pthread_rwlock_trywrlock.c |
| `__pthread_rwlock_timedwrlock` | `pthread_rwlock_timedwrlock` | pthread_rwlock_timedwrlock.c |
| `__pthread_rwlock_unlock` | `pthread_rwlock_unlock` | pthread_rwlock_unlock.c |

### 19.2 mutex

| 内部符号 | 对外导出符号 | 源文件 |
|----------|-------------|--------|
| `__pthread_mutex_lock` | `pthread_mutex_lock` | pthread_mutex_lock.c |
| `__pthread_mutex_trylock` | `pthread_mutex_trylock` | pthread_mutex_trylock.c |
| `__pthread_mutex_timedlock` | `pthread_mutex_timedlock` | pthread_mutex_timedlock.c |
| `__pthread_mutex_unlock` | `pthread_mutex_unlock` | pthread_mutex_unlock.c |

### 19.3 其他

| 内部符号 | 对外导出符号 | 源文件 |
|----------|-------------|--------|
| `__pthread_once` | `pthread_once` | pthread_once.c |
| `__pthread_testcancel` | `pthread_testcancel` | pthread_testcancel.c |
| `__pthread_setcancelstate` | `pthread_setcancelstate` | pthread_setcancelstate.c |
| `__pthread_self_internal` | `pthread_self`, `thrd_current` | pthread_self.c |
| `__pthread_equal` | `pthread_equal`, `thrd_equal` | pthread_equal.c |
| `__pthread_key_create` | `pthread_key_create` | pthread_key_create.c |
| `__pthread_key_delete` | `pthread_key_delete` | pthread_key_create.c |
| `__pthread_getspecific` | `pthread_getspecific`, `tss_get` | pthread_getspecific.c |

---

## 20. 内部导出符号（被其他模块使用）

以下符号不通过标准头文件暴露给用户程序，但被 musl 其他内部模块引用。

| 符号 | 类型 | 说明 | 源文件 |
|------|------|------|------|
| `__pthread_tsd_size` | `volatile size_t` | TSD 数组总字节大小 | pthread_key_create.c |
| `__pthread_tsd_main` | `void *[]` | 主线程默认 TSD 数组 | pthread_key_create.c |
| `__pthread_tsd_run_dtors` | `void (*)(void)` | 线程退出时运行 TSD 析构函数 | pthread_key_create.c |
| `__sem_open_lockptr` | `volatile int *const` | semtab 锁指针 | sem_open.c |
| `__fork_handler` | `void (*)(int)` | fork 处理回调调度 | pthread_atfork.c |
| `__cancel` | `long (*)(void)` | 线程取消退出 | pthread_cancel.c |
| `__syscall_cp_c` | `long (*)(syscall_arg_t, ...)` | 取消点系统调用包装 | pthread_cancel.c |
| `__syscall_cp_asm` | `long (*)(volatile void *, ...)` | 取消点系统调用汇编包装 | pthread_cancel.c |

---

## 21. 导出常量

### 21.1 线程属性常量

| 宏 | 值 | 说明 |
|----|-----|------|
| `PTHREAD_CREATE_JOINABLE` | 0 | joinable 线程 |
| `PTHREAD_CREATE_DETACHED` | 1 | detached 线程 |
| `PTHREAD_INHERIT_SCHED` | 0 | 继承调度属性 |
| `PTHREAD_EXPLICIT_SCHED` | 1 | 显式调度属性 |
| `PTHREAD_SCOPE_SYSTEM` | 0 | 系统级竞争范围 |
| `PTHREAD_SCOPE_PROCESS` | 1 | 进程级竞争范围 |
| `PTHREAD_STACK_MIN` | 2048 | 最小线程栈大小 |

### 21.2 互斥锁常量

| 宏 | 值 | 说明 |
|----|-----|------|
| `PTHREAD_MUTEX_NORMAL` | 0 | 普通锁 |
| `PTHREAD_MUTEX_DEFAULT` | 0 | `PTHREAD_MUTEX_NORMAL` 的同义名 |
| `PTHREAD_MUTEX_RECURSIVE` | 1 | 允许同一线程重复加锁 |
| `PTHREAD_MUTEX_ERRORCHECK` | 2 | 检测死锁 |
| `PTHREAD_MUTEX_STALLED` | 0 | 非健壮互斥锁（默认） |
| `PTHREAD_MUTEX_ROBUST` | 1 | 健壮互斥锁 |
| `PTHREAD_PRIO_NONE` | 0 | 无优先级协议 |
| `PTHREAD_PRIO_INHERIT` | 1 | 优先级继承 |
| `PTHREAD_PRIO_PROTECT` | 2 | 优先级保护（musl 不支持） |
| `PTHREAD_MUTEX_INITIALIZER` | `{{{0}}}` | 静态初始化器 |

### 21.3 取消常量

| 宏 | 值 | 说明 |
|----|-----|------|
| `PTHREAD_CANCEL_ENABLE` | 0 | 取消启用 |
| `PTHREAD_CANCEL_DISABLE` | 1 | 取消禁用 |
| `PTHREAD_CANCEL_MASKED` | 2 | 取消屏蔽（内部） |
| `PTHREAD_CANCEL_DEFERRED` | 0 | 延迟取消 |
| `PTHREAD_CANCEL_ASYNCHRONOUS` | 1 | 异步取消 |
| `PTHREAD_CANCELED` | `((void *)-1)` | 线程被取消时的返回值 |

### 21.4 通用常量

| 宏 | 值 | 说明 |
|----|-----|------|
| `PTHREAD_ONCE_INIT` | 0 | pthread_once_t 静态初始化器 |
| `PTHREAD_PROCESS_PRIVATE` | 0 | 进程内共享 |
| `PTHREAD_PROCESS_SHARED` | 1 | 跨进程共享 |
| `PTHREAD_NULL` | `((pthread_t)0)` | 空线程标识符 |
| `PTHREAD_RWLOCK_INITIALIZER` | `{{{0}}}` | 读写锁静态初始化器 |
| `PTHREAD_KEYS_MAX` | 128 | 最大 TSD 键数 |
| `PTHREAD_DESTRUCTOR_ITERATIONS` | 4 | TSD 析构最大轮数 |
| `SIGCANCEL` | 33 | 内部取消信号号 |
| `SIGSYNCCALL` | 34 | 内部同步调用信号号 |

### 21.5 信号量常量

| 宏 | 值 | 说明 |
|----|-----|------|
| `SEM_VALUE_MAX` | 0x7FFFFFFF | 信号量最大计数值 |
| `SEM_NSEMS_MAX` | 256 | 最大同时打开有名信号量数 |
| `SEM_FAILED` | `((sem_t *)0)` | sem_open 失败返回值 |

### 21.6 C11 线程常量

| 符号 | 值 | 说明 |
|------|-----|------|
| `thrd_success` | 0 | 操作成功 |
| `thrd_busy` | 1 | 资源忙 |
| `thrd_error` | 2 | 操作失败 |
| `thrd_nomem` | 3 | 内存不足 |
| `thrd_timedout` | 4 | 超时 |
| `mtx_plain` | 0 | 普通互斥锁 |
| `mtx_recursive` | 1 | 递归互斥锁 |
| `mtx_timed` | 2 | 支持超时的互斥锁（musl 中忽略） |
| `ONCE_FLAG_INIT` | 0 | once_flag 初始化器 |
| `TSS_DTOR_ITERATIONS` | 4 | TSS 析构最大迭代次数 |

### 21.7 C11 线程类型别名

| 类型 | 底层类型 | 说明 |
|------|----------|------|
| `thrd_t` | `struct __pthread *` (C) / `unsigned long` (C++) | 线程标识符 |
| `thrd_start_t` | `int (*)(void *)` | 线程入口函数类型 |
| `tss_t` | `unsigned` | TSS 键 |
| `tss_dtor_t` | `void (*)(void *)` | TSS 析构函数类型 |
| `once_flag` | `int` | 一次性执行标志 |
| `cnd_t` | struct (同 `pthread_cond_t`) | 条件变量 |
| `mtx_t` | struct (同 `pthread_mutex_t`) | 互斥锁 |
