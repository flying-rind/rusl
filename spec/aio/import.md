# aio 模块 — 外部依赖清单

> 本文件记录 `musl-1.2.6/src/aio/` 模块内所有 C 源文件使用的外部模块 C 接口。

---

## 来自 `stdlib` (libc-stdlib)

| 接口 | 声明位置 | 使用文件 |
|------|----------|----------|
| `malloc` / `__libc_malloc` | `<stdlib.h>` | `lio_listio.c`, `aio.c` |
| `free` / `__libc_free` | `<stdlib.h>` | `lio_listio.c`, `aio.c` |
| `calloc` / `__libc_calloc` | `<stdlib.h>` | `aio.c` |
| `realloc` / `__libc_realloc` | `<stdlib.h>` | `aio.c` |

## 来自 `unistd` (libc-unistd)

| 接口 | 声明位置 | 使用文件 |
|------|----------|----------|
| `read` | `<unistd.h>` | `aio.c` |
| `write` | `<unistd.h>` | `aio.c` |
| `pread` | `<unistd.h>` | `aio.c` |
| `pwrite` | `<unistd.h>` | `aio.c` |
| `fsync` | `<unistd.h>` | `aio.c` |
| `fdatasync` | `<unistd.h>` | `aio.c` |
| `lseek` | `<unistd.h>` | `aio.c` |
| `fcntl` | `<unistd.h>` | `aio.c` |
| `getpid` | `<unistd.h>` | `lio_listio.c`, `aio.c` |
| `getuid` | `<unistd.h>` | `lio_listio.c`, `aio.c` |
| `close` | `<unistd.h>` | (通过 `__aio_close` 间接使用) |

## 来自 `string` (libc-string)

| 接口 | 声明位置 | 使用文件 |
|------|----------|----------|
| `memcpy` | `<string.h>` | `lio_listio.c` |

## 来自 `pthread` (libc-pthread)

| 接口 | 声明位置 | 使用文件 |
|------|----------|----------|
| `pthread_create` | `<pthread.h>` | `lio_listio.c`, `aio.c` |
| `pthread_cancel` | `<pthread.h>` | `aio.c` |
| `pthread_attr_init` | `<pthread.h>` | `lio_listio.c`, `aio.c` |
| `pthread_attr_setstacksize` | `<pthread.h>` | `lio_listio.c`, `aio.c` |
| `pthread_attr_setguardsize` | `<pthread.h>` | `lio_listio.c`, `aio.c` |
| `pthread_attr_setdetachstate` | `<pthread.h>` | `lio_listio.c`, `aio.c` |
| `pthread_mutex_lock` | `<pthread.h>` | `aio.c` |
| `pthread_mutex_unlock` | `<pthread.h>` | `aio.c` |
| `pthread_mutex_init` | `<pthread.h>` | `aio.c` |
| `pthread_cond_wait` | `<pthread.h>` | `aio.c` |
| `pthread_cond_broadcast` | `<pthread.h>` | `aio.c` |
| `pthread_cond_init` | `<pthread.h>` | `aio.c` |
| `pthread_rwlock_rdlock` | `<pthread.h>` | `aio.c` |
| `pthread_rwlock_wrlock` | `<pthread.h>` | `aio.c` |
| `pthread_rwlock_unlock` | `<pthread.h>` | `aio.c` |
| `pthread_rwlock_init` | `<pthread.h>` | `aio.c` |
| `pthread_sigmask` | `<signal.h>` | `lio_listio.c`, `aio.c` |
| `pthread_cleanup_push` / `pthread_cleanup_pop` | `<pthread.h>` | `aio.c` |
| `pthread_testcancel` | `<pthread.h>` | `aio_suspend.c` |
| `pthread_self` (via `__pthread_self`) | `pthread_impl.h` | `aio.c` |
| `PTHREAD_CREATE_DETACHED` | `<pthread.h>` | `lio_listio.c`, `aio.c` |
| `PTHREAD_RWLOCK_INITIALIZER` | `<pthread.h>` | `aio.c` |

## 来自 `signal` (libc-signal)

| 接口 | 声明位置 | 使用文件 |
|------|----------|----------|
| `siginfo_t` | `<signal.h>` | `lio_listio.c`, `aio.c` |
| `struct sigevent` | `<signal.h>` (包含) | `lio_listio.c`, `aio.c` |
| `sigset_t` | `<signal.h>` | `lio_listio.c`, `aio.c` |
| `sigfillset` | `<signal.h>` | `lio_listio.c`, `aio.c` |
| `SIG_BLOCK` | `<signal.h>` | `lio_listio.c`, `aio.c` |
| `SIG_SETMASK` | `<signal.h>` | `lio_listio.c`, `aio.c` |
| `SI_ASYNCIO` | `<signal.h>` | `lio_listio.c`, `aio.c` |
| `SIGEV_NONE` | `<signal.h>` | `lio_listio.c`, `aio.c` |
| `SIGEV_SIGNAL` | `<signal.h>` | `lio_listio.c`, `aio.c` |
| `SIGEV_THREAD` | `<signal.h>` | `lio_listio.c`, `aio.c` |

## 来自 `semaphore` (libc-pthread)

| 接口 | 声明位置 | 使用文件 |
|------|----------|----------|
| `sem_t` | `<semaphore.h>` | `aio.c` |
| `sem_init` | `<semaphore.h>` | `aio.c` |
| `sem_post` | `<semaphore.h>` | `aio.c` |
| `sem_wait` | `<semaphore.h>` | `aio.c` |

## 来自 `time` (libc-time)

| 接口 | 声明位置 | 使用文件 |
|------|----------|----------|
| `struct timespec` | `<time.h>` | `aio_suspend.c` |
| `clock_gettime` | `<time.h>` | `aio_suspend.c` |
| `CLOCK_MONOTONIC` | `<time.h>` | `aio_suspend.c` |

---

## 来自 musl 内部模块 (internal)

| 接口 | 声明位置 | 使用文件 |
|------|----------|----------|
| `__syscall` | `internal/syscall.h` | `lio_listio.c`, `aio.c` |
| `SYS_rt_sigqueueinfo` | `internal/syscall.h` (架构相关) | `lio_listio.c`, `aio.c` |
| `SYS_futex` | `internal/syscall.h` | `aio.c` |
| `__syscall_ret` | `internal/syscall.h` | `aio.c` |
| `a_inc`, `a_dec`, `a_swap`, `a_cas`, `a_barrier`, `a_store` | `internal/atomic.h` | `aio.c`, `aio_suspend.c` |
| `__wake`, `__futexwait`, `__wait`, `__timedwait_cp` | `internal/pthread_impl.h` | `aio.c`, `aio_suspend.c` |
| `FUTEX_WAKE`, `FUTEX_WAIT`, `FUTEX_PRIVATE` | `internal/futex.h` | `aio.c` |
| `__getauxval` | `internal/libc.h` | `aio.c` |
| `__pthread_self` | `internal/pthread_impl.h` | `aio.c` |
| `AT_MINSIGSTKSZ` | `<sys/auxv.h>` | `aio.c` |
| `MINSIGSTKSZ` | `<signal.h>` | `aio.c` |
| `PAGE_SIZE` | `<limits.h>` | `lio_listio.c` |
