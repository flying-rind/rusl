# pthread_rwlockattr_setpshared.c 规约

> musl libc 读写锁属性设置函数。设置读写锁是否可在进程间共享。

---

## 依赖图

```
pthread_rwlockattr_setpshared (Public API)
  (无内部函数依赖)
```

---

## 函数规约

### 1. pthread_rwlockattr_setpshared

```c
int pthread_rwlockattr_setpshared(pthread_rwlockattr_t *a, int pshared);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出（POSIX）

#### Intent

设置读写锁的进程共享属性。控制由该属性对象初始化的读写锁是否可在多个进程之间共享（通过共享内存）。

#### 前置条件

- `a != NULL`，指向一个已初始化的有效 `pthread_rwlockattr_t` 对象
- `pshared` 必须为以下值之一：
  - `PTHREAD_PROCESS_PRIVATE` (0): 读写锁仅在同一进程内的线程间共享
  - `PTHREAD_PROCESS_SHARED` (1): 读写锁可在进程间共享

#### 后置条件

- Case 1 成功（`pshared` 为 0 或 1）：
  - `a->__attr[0] = pshared`
  - 返回 0
- Case 2 失败（`pshared` 无效）：返回 `EINVAL`

#### 不变量

无。

#### 系统算法

```
pthread_rwlockattr_setpshared(a, pshared):
  1. if (pshared > 1U) return EINVAL       仅接受 0 或 1
  2. a->__attr[0] = pshared                存储进程共享标志
  3. return 0
```

与 `PTHREAD_PROCESS_SHARED` 的兼容性说明：musl 将 `__attr[0]` 作为原始值存储。在 `pthread_rwlock_init` 中，该值乘以 128 后存入 `rw->_rw_shared`，用于后续 futex 操作的 `FUTEX_PRIVATE` 标志控制。

#### 依赖

- `EINVAL` — 来自 `<errno.h>` 的错误码常量
- `pthread_rwlockattr_t` — 来自 `<alltypes.h.in>` 的类型定义
