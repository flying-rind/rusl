# pthread_attr_get.c 规约

> musl libc 线程属性获取函数集合。从 `pthread_attr_t` 对象中读取各项属性值。同时包含 barrier、cond、mutex、rwlock 属性获取函数。

---

## 依赖图

```
pthread_attr_getdetachstate
pthread_attr_getguardsize
pthread_attr_getinheritsched
pthread_attr_getschedparam
pthread_attr_getschedpolicy
pthread_attr_getscope
pthread_attr_getstack
pthread_attr_getstacksize
pthread_barrierattr_getpshared
pthread_condattr_getclock
pthread_condattr_getpshared
pthread_mutexattr_getprotocol
pthread_mutexattr_getpshared
pthread_mutexattr_getrobust
pthread_mutexattr_gettype
pthread_rwlockattr_getpshared
  (上述函数均为纯读取操作，无外部函数调用依赖)
```

---

## 函数规约

### 1. pthread_attr_getdetachstate

```c
int pthread_attr_getdetachstate(const pthread_attr_t *a, int *state);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

从线程属性对象中获取分离状态（joinable 或 detached）。

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`
- `state != NULL`

#### 后置条件

- Case 1 始终成功：返回 `0`
- `*state` = `a->_a_detach`（0 = `PTHREAD_CREATE_JOINABLE`, 1 = `PTHREAD_CREATE_DETACHED`）

#### 不变量

该函数为纯读取操作，不修改任何全局状态。

---

### 2. pthread_attr_getguardsize

```c
int pthread_attr_getguardsize(const pthread_attr_t *restrict a, size_t *restrict size);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

从线程属性对象中获取守护页大小。

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`
- `size != NULL`

#### 后置条件

- Case 1 始终成功：返回 `0`
- `*size` = `a->_a_guardsize`

#### 不变量

该函数为纯读取操作，不修改任何全局状态。

---

### 3. pthread_attr_getinheritsched

```c
int pthread_attr_getinheritsched(const pthread_attr_t *restrict a, int *restrict inherit);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

获取线程属性中的调度继承策略。

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`
- `inherit != NULL`

#### 后置条件

- Case 1 始终成功：返回 `0`
- `*inherit` = `a->_a_sched`（0 = `PTHREAD_INHERIT_SCHED`, 1 = `PTHREAD_EXPLICIT_SCHED`）

#### 不变量

该函数为纯读取操作，不修改任何全局状态。

---

### 4. pthread_attr_getschedparam

```c
int pthread_attr_getschedparam(const pthread_attr_t *restrict a, struct sched_param *restrict param);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

获取线程属性中的调度参数（优先级）。

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`
- `param != NULL`

#### 后置条件

- Case 1 始终成功：返回 `0`
- `param->sched_priority` = `a->_a_prio`

#### 不变量

该函数为纯读取操作，不修改任何全局状态。

---

### 5. pthread_attr_getschedpolicy

```c
int pthread_attr_getschedpolicy(const pthread_attr_t *restrict a, int *restrict policy);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

获取线程属性中的调度策略（如 SCHED_FIFO、SCHED_RR、SCHED_OTHER）。

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`
- `policy != NULL`

#### 后置条件

- Case 1 始终成功：返回 `0`
- `*policy` = `a->_a_policy`

#### 不变量

该函数为纯读取操作，不修改任何全局状态。

---

### 6. pthread_attr_getscope

```c
int pthread_attr_getscope(const pthread_attr_t *restrict a, int *restrict scope);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

获取线程属性中的竞争范围。musl（Linux）仅支持 `PTHREAD_SCOPE_SYSTEM`。

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`
- `scope != NULL`

#### 后置条件

- Case 1 始终成功：返回 `0`
- `*scope` = `PTHREAD_SCOPE_SYSTEM`（固定返回值，不从 attr 中读取）

#### 不变量

musl/Linux 下只支持系统级竞争范围，该函数始终返回 `PTHREAD_SCOPE_SYSTEM`。

---

### 7. pthread_attr_getstack

```c
int pthread_attr_getstack(const pthread_attr_t *restrict a, void **restrict addr, size_t *restrict size);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

获取线程属性中设置的栈地址和栈大小。与 `pthread_attr_getstacksize` 不同，此函数会同时返回栈的地址。

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`
- `addr != NULL`
- `size != NULL`

#### 后置条件

- Case 1 栈地址已设置（`a->_a_stackaddr != 0`）：
  - `*size` = `a->_a_stacksize`
  - `*addr` = `(void *)(a->_a_stackaddr - *size)` — 返回栈基址（栈顶向低地址增长）
  - 返回 `0`
- Case 2 栈地址未设置（`a->_a_stackaddr == 0`）：
  - 返回 `EINVAL`

#### 不变量

该函数为纯读取操作，不修改任何全局状态。

---

### 8. pthread_attr_getstacksize

```c
int pthread_attr_getstacksize(const pthread_attr_t *restrict a, size_t *restrict size);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

获取线程属性中设置的栈大小。

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`
- `size != NULL`

#### 后置条件

- Case 1 始终成功：返回 `0`
- `*size` = `a->_a_stacksize`

#### 不变量

该函数为纯读取操作，不修改任何全局状态。

---

### 9. pthread_barrierattr_getpshared

```c
int pthread_barrierattr_getpshared(const pthread_barrierattr_t *restrict a, int *restrict pshared);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

获取 barrier 属性中的进程共享标志。

#### 前置条件

- `a != NULL`
- `pshared != NULL`

#### 后置条件

- Case 1 始终成功：返回 `0`
- `*pshared` = `!!a->__attr`（第 0 位取布尔值：0 = `PTHREAD_PROCESS_PRIVATE`, 1 = `PTHREAD_PROCESS_SHARED`）

#### 不变量

该函数为纯读取操作。

---

### 10. pthread_condattr_getclock

```c
int pthread_condattr_getclock(const pthread_condattr_t *restrict a, clockid_t *restrict clk);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

获取条件变量属性中设置的时钟类型。

#### 前置条件

- `a != NULL`
- `clk != NULL`

#### 后置条件

- Case 1 始终成功：返回 `0`
- `*clk` = `a->__attr & 0x7fffffff`（低 31 位为 clockid）

#### 不变量

该函数为纯读取操作。

---

### 11. pthread_condattr_getpshared

```c
int pthread_condattr_getpshared(const pthread_condattr_t *restrict a, int *restrict pshared);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

获取条件变量属性中的进程共享标志。

#### 前置条件

- `a != NULL`
- `pshared != NULL`

#### 后置条件

- Case 1 始终成功：返回 `0`
- `*pshared` = `a->__attr >> 31`（最高位：0 = private, 1 = shared）

#### 不变量

该函数为纯读取操作。

---

### 12. pthread_mutexattr_getprotocol

```c
int pthread_mutexattr_getprotocol(const pthread_mutexattr_t *restrict a, int *restrict protocol);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

获取互斥锁属性中的优先级协议。

#### 前置条件

- `a != NULL`
- `protocol != NULL`

#### 后置条件

- Case 1 始终成功：返回 `0`
- `*protocol` = `a->__attr / 8U % 2`（第 3 位：0 = `PTHREAD_PRIO_NONE`, 1 = `PTHREAD_PRIO_INHERIT`）

#### 不变量

该函数为纯读取操作。

---

### 13. pthread_mutexattr_getpshared

```c
int pthread_mutexattr_getpshared(const pthread_mutexattr_t *restrict a, int *restrict pshared);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

获取互斥锁属性中的进程共享标志。

#### 前置条件

- `a != NULL`
- `pshared != NULL`

#### 后置条件

- Case 1 始终成功：返回 `0`
- `*pshared` = `a->__attr / 128U % 2`（第 7 位）

#### 不变量

该函数为纯读取操作。

---

### 14. pthread_mutexattr_getrobust

```c
int pthread_mutexattr_getrobust(const pthread_mutexattr_t *restrict a, int *restrict robust);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

获取互斥锁属性中的健壮性标志。

#### 前置条件

- `a != NULL`
- `robust != NULL`

#### 后置条件

- Case 1 始终成功：返回 `0`
- `*robust` = `a->__attr / 4U % 2`（第 2 位：0 = `PTHREAD_MUTEX_STALLED`, 1 = `PTHREAD_MUTEX_ROBUST`）

#### 不变量

该函数为纯读取操作。

---

### 15. pthread_mutexattr_gettype

```c
int pthread_mutexattr_gettype(const pthread_mutexattr_t *restrict a, int *restrict type);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

获取互斥锁属性中的类型（NORMAL/RECURSIVE/ERRORCHECK）。

#### 前置条件

- `a != NULL`
- `type != NULL`

#### 后置条件

- Case 1 始终成功：返回 `0`
- `*type` = `a->__attr & 3`（低 2 位：0 = NORMAL, 1 = RECURSIVE, 2 = ERRORCHECK）

#### 不变量

该函数为纯读取操作。

---

### 16. pthread_rwlockattr_getpshared

```c
int pthread_rwlockattr_getpshared(const pthread_rwlockattr_t *restrict a, int *restrict pshared);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

获取读写锁属性中的进程共享标志。

#### 前置条件

- `a != NULL`
- `pshared != NULL`

#### 后置条件

- Case 1 始终成功：返回 `0`
- `*pshared` = `a->__attr[0]`（第 0 个属性元素）

#### 不变量

该函数为纯读取操作。

---

#### 依赖

- `pthread_impl.h` — 内部头文件，定义所有 `_a_xxx`/`__attr` 成员访问宏
- `PTHREAD_SCOPE_SYSTEM` — 宏，定义于 `<pthread.h>`
- `EINVAL` — 宏，定义于 `<errno.h>`
- `struct sched_param` — 定义于 `<sched.h>`
- `clockid_t` — 定义于 `<time.h>`
