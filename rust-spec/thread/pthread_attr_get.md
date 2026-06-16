# pthread_attr_get — Rust 接口归约

## 原始 C 接口

```c
int pthread_attr_getdetachstate(const pthread_attr_t *a, int *state);
int pthread_attr_getguardsize(const pthread_attr_t *restrict a, size_t *restrict size);
int pthread_attr_getinheritsched(const pthread_attr_t *restrict a, int *restrict inherit);
int pthread_attr_getschedparam(const pthread_attr_t *restrict a, struct sched_param *restrict param);
int pthread_attr_getschedpolicy(const pthread_attr_t *restrict a, int *restrict policy);
int pthread_attr_getscope(const pthread_attr_t *restrict a, int *restrict scope);
int pthread_attr_getstack(const pthread_attr_t *restrict a, void **restrict addr, size_t *restrict size);
int pthread_attr_getstacksize(const pthread_attr_t *restrict a, size_t *restrict size);
int pthread_barrierattr_getpshared(const pthread_barrierattr_t *restrict a, int *restrict pshared);
int pthread_condattr_getclock(const pthread_condattr_t *restrict a, clockid_t *restrict clk);
int pthread_condattr_getpshared(const pthread_condattr_t *restrict a, int *restrict pshared);
int pthread_mutexattr_getprotocol(const pthread_mutexattr_t *restrict a, int *restrict protocol);
int pthread_mutexattr_getpshared(const pthread_mutexattr_t *restrict a, int *restrict pshared);
int pthread_mutexattr_getrobust(const pthread_mutexattr_t *restrict a, int *restrict robust);
int pthread_mutexattr_gettype(const pthread_mutexattr_t *restrict a, int *restrict type);
int pthread_rwlockattr_getpshared(const pthread_rwlockattr_t *restrict a, int *restrict pshared);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_attr_getdetachstate(
    a: *const pthread_attr_t,
    state: *mut core::ffi::c_int,
) -> core::ffi::c_int;

extern "C" fn pthread_attr_getguardsize(
    a: *const pthread_attr_t,
    size: *mut usize,
) -> core::ffi::c_int;

extern "C" fn pthread_attr_getinheritsched(
    a: *const pthread_attr_t,
    inherit: *mut core::ffi::c_int,
) -> core::ffi::c_int;

extern "C" fn pthread_attr_getschedparam(
    a: *const pthread_attr_t,
    param: *mut sched_param,
) -> core::ffi::c_int;

extern "C" fn pthread_attr_getschedpolicy(
    a: *const pthread_attr_t,
    policy: *mut core::ffi::c_int,
) -> core::ffi::c_int;

extern "C" fn pthread_attr_getscope(
    a: *const pthread_attr_t,
    scope: *mut core::ffi::c_int,
) -> core::ffi::c_int;

extern "C" fn pthread_attr_getstack(
    a: *const pthread_attr_t,
    addr: *mut *mut core::ffi::c_void,
    size: *mut usize,
) -> core::ffi::c_int;

extern "C" fn pthread_attr_getstacksize(
    a: *const pthread_attr_t,
    size: *mut usize,
) -> core::ffi::c_int;

extern "C" fn pthread_barrierattr_getpshared(
    a: *const pthread_barrierattr_t,
    pshared: *mut core::ffi::c_int,
) -> core::ffi::c_int;

extern "C" fn pthread_condattr_getclock(
    a: *const pthread_condattr_t,
    clk: *mut clockid_t,
) -> core::ffi::c_int;

extern "C" fn pthread_condattr_getpshared(
    a: *const pthread_condattr_t,
    pshared: *mut core::ffi::c_int,
) -> core::ffi::c_int;

extern "C" fn pthread_mutexattr_getprotocol(
    a: *const pthread_mutexattr_t,
    protocol: *mut core::ffi::c_int,
) -> core::ffi::c_int;

extern "C" fn pthread_mutexattr_getpshared(
    a: *const pthread_mutexattr_t,
    pshared: *mut core::ffi::c_int,
) -> core::ffi::c_int;

extern "C" fn pthread_mutexattr_getrobust(
    a: *const pthread_mutexattr_t,
    robust: *mut core::ffi::c_int,
) -> core::ffi::c_int;

extern "C" fn pthread_mutexattr_gettype(
    a: *const pthread_mutexattr_t,
    type_: *mut core::ffi::c_int,
) -> core::ffi::c_int;

extern "C" fn pthread_rwlockattr_getpshared(
    a: *const pthread_rwlockattr_t,
    pshared: *mut core::ffi::c_int,
) -> core::ffi::c_int;
```

---

## 意图

从线程/同步对象的属性对象中读取各项属性值。所有函数均为纯读取操作，不修改全局状态和属性对象本身。

## 函数规约

### 1. pthread_attr_getdetachstate

#### 前置条件
- `a` 为非空指针
- `state` 为非空指针

#### 后置条件
- Case 1 始终成功：返回 `0`
- `*state` = `a._a_detach`（0 = `PTHREAD_CREATE_JOINABLE`, 1 = `PTHREAD_CREATE_DETACHED`）

---

### 2. pthread_attr_getguardsize

#### 前置条件
- `a` 为非空指针
- `size` 为非空指针

#### 后置条件
- Case 1 始终成功：返回 `0`
- `*size` = `a._a_guardsize`

---

### 3. pthread_attr_getinheritsched

#### 前置条件
- `a` 为非空指针
- `inherit` 为非空指针

#### 后置条件
- Case 1 始终成功：返回 `0`
- `*inherit` = `a._a_sched`（0 = `PTHREAD_INHERIT_SCHED`, 1 = `PTHREAD_EXPLICIT_SCHED`）

---

### 4. pthread_attr_getschedparam

#### 前置条件
- `a` 为非空指针
- `param` 为非空指针
- `a` 与 `param` 不重叠

#### 后置条件
- Case 1 始终成功：返回 `0`
- `param.sched_priority` = `a._a_prio`

---

### 5. pthread_attr_getschedpolicy

#### 前置条件
- `a` 为非空指针
- `policy` 为非空指针

#### 后置条件
- Case 1 始终成功：返回 `0`
- `*policy` = `a._a_policy`

---

### 6. pthread_attr_getscope

#### 前置条件
- `a` 为非空指针
- `scope` 为非空指针

#### 后置条件
- Case 1 始终成功：返回 `0`
- `*scope` = `PTHREAD_SCOPE_SYSTEM`（固定返回值，不从 attr 中读取）

---

### 7. pthread_attr_getstack

#### 前置条件
- `a` 为非空指针
- `addr` 为非空指针
- `size` 为非空指针

#### 后置条件
- Case 1 栈地址已设置（`a._a_stackaddr != 0`）：
  - `*size` = `a._a_stacksize`
  - `*addr` = `(void *)(a._a_stackaddr - *size)` — 返回栈基址（栈向低地址增长）
  - 返回 `0`
- Case 2 栈地址未设置（`a._a_stackaddr == 0`）：
  - 返回 `EINVAL`

---

### 8. pthread_attr_getstacksize

#### 前置条件
- `a` 为非空指针
- `size` 为非空指针

#### 后置条件
- Case 1 始终成功：返回 `0`
- `*size` = `a._a_stacksize`

---

### 9. pthread_barrierattr_getpshared

#### 前置条件
- `a` 为非空指针
- `pshared` 为非空指针

#### 后置条件
- Case 1 始终成功：返回 `0`
- `*pshared` = `!!a.__attr`（0 = `PTHREAD_PROCESS_PRIVATE`, 1 = `PTHREAD_PROCESS_SHARED`）

---

### 10. pthread_condattr_getclock

#### 前置条件
- `a` 为非空指针
- `clk` 为非空指针

#### 后置条件
- Case 1 始终成功：返回 `0`
- `*clk` = `a.__attr & 0x7fffffff`（低 31 位为 clockid）

---

### 11. pthread_condattr_getpshared

#### 前置条件
- `a` 为非空指针
- `pshared` 为非空指针

#### 后置条件
- Case 1 始终成功：返回 `0`
- `*pshared` = `a.__attr >> 31`（最高位：0 = private, 1 = shared）

---

### 12. pthread_mutexattr_getprotocol

#### 前置条件
- `a` 为非空指针
- `protocol` 为非空指针

#### 后置条件
- Case 1 始终成功：返回 `0`
- `*protocol` = `(a.__attr / 8) % 2`（第 3 位：0 = `PTHREAD_PRIO_NONE`, 1 = `PTHREAD_PRIO_INHERIT`）

---

### 13. pthread_mutexattr_getpshared

#### 前置条件
- `a` 为非空指针
- `pshared` 为非空指针

#### 后置条件
- Case 1 始终成功：返回 `0`
- `*pshared` = `(a.__attr / 128) % 2`（第 7 位）

---

### 14. pthread_mutexattr_getrobust

#### 前置条件
- `a` 为非空指针
- `robust` 为非空指针

#### 后置条件
- Case 1 始终成功：返回 `0`
- `*robust` = `(a.__attr / 4) % 2`（第 2 位：0 = `PTHREAD_MUTEX_STALLED`, 1 = `PTHREAD_MUTEX_ROBUST`）

---

### 15. pthread_mutexattr_gettype

#### 前置条件
- `a` 为非空指针
- `type_` 为非空指针

#### 后置条件
- Case 1 始终成功：返回 `0`
- `*type_` = `a.__attr & 3`（低 2 位：0 = NORMAL, 1 = RECURSIVE, 2 = ERRORCHECK）

---

### 16. pthread_rwlockattr_getpshared

#### 前置条件
- `a` 为非空指针
- `pshared` 为非空指针

#### 后置条件
- Case 1 始终成功：返回 `0`
- `*pshared` = `a.__attr[0]`

---

## 不变量

所有函数均为纯读取操作，不修改属性对象和任何全局状态。

## 算法

每个 get 函数内部：
1. 通过 `unsafe` 块解引用 `a` 读取对应字段
2. 将读取的值写入 `*out` 参数
3. 返回 `0`（或 `EINVAL` 对于 `pthread_attr_getstack` 未设置栈地址的情况）

对于 `pthread_attr_getscope`，硬编码返回 `PTHREAD_SCOPE_SYSTEM`（不读取 `a` 的任何字段），因为 musl/Linux 仅支持系统级竞争范围。

Rust 实现中这些函数可以用简单的字段访问实现，无需外部调用。

---

/* Rely */
[RELY]
Predefined Structures:
  pthread_attr_t          // 定义于 pthread_impl 模块
  pthread_barrierattr_t   // 定义于 pthread_impl 模块
  pthread_condattr_t      // 定义于 pthread_impl 模块
  pthread_mutexattr_t     // 定义于 pthread_impl 模块
  pthread_rwlockattr_t    // 定义于 pthread_impl 模块
  sched_param             // 定义于 <sched.h>
Predefined Types:
  clockid_t               // 定义于 <time.h>
Predefined Macros/Constants:
  PTHREAD_CREATE_JOINABLE     = 0
  PTHREAD_CREATE_DETACHED     = 1
  PTHREAD_INHERIT_SCHED       = 0
  PTHREAD_EXPLICIT_SCHED      = 1
  PTHREAD_SCOPE_SYSTEM        = 0
  PTHREAD_PROCESS_PRIVATE     = 0
  PTHREAD_PROCESS_SHARED      = 1
  PTHREAD_PRIO_NONE           = 0
  PTHREAD_PRIO_INHERIT        = 1
  PTHREAD_MUTEX_STALLED       = 0
  PTHREAD_MUTEX_ROBUST        = 1
  PTHREAD_MUTEX_NORMAL        = 0
  PTHREAD_MUTEX_RECURSIVE     = 1
  PTHREAD_MUTEX_ERRORCHECK    = 2
  EINVAL                      // 定义于 <errno.h>

[GUARANTEE]
Exported Interfaces:
  extern "C" fn pthread_attr_getdetachstate(a: *const pthread_attr_t, state: *mut c_int) -> c_int;
  extern "C" fn pthread_attr_getguardsize(a: *const pthread_attr_t, size: *mut usize) -> c_int;
  extern "C" fn pthread_attr_getinheritsched(a: *const pthread_attr_t, inherit: *mut c_int) -> c_int;
  extern "C" fn pthread_attr_getschedparam(a: *const pthread_attr_t, param: *mut sched_param) -> c_int;
  extern "C" fn pthread_attr_getschedpolicy(a: *const pthread_attr_t, policy: *mut c_int) -> c_int;
  extern "C" fn pthread_attr_getscope(a: *const pthread_attr_t, scope: *mut c_int) -> c_int;
  extern "C" fn pthread_attr_getstack(a: *const pthread_attr_t, addr: *mut *mut c_void, size: *mut usize) -> c_int;
  extern "C" fn pthread_attr_getstacksize(a: *const pthread_attr_t, size: *mut usize) -> c_int;
  extern "C" fn pthread_barrierattr_getpshared(a: *const pthread_barrierattr_t, pshared: *mut c_int) -> c_int;
  extern "C" fn pthread_condattr_getclock(a: *const pthread_condattr_t, clk: *mut clockid_t) -> c_int;
  extern "C" fn pthread_condattr_getpshared(a: *const pthread_condattr_t, pshared: *mut c_int) -> c_int;
  extern "C" fn pthread_mutexattr_getprotocol(a: *const pthread_mutexattr_t, protocol: *mut c_int) -> c_int;
  extern "C" fn pthread_mutexattr_getpshared(a: *const pthread_mutexattr_t, pshared: *mut c_int) -> c_int;
  extern "C" fn pthread_mutexattr_getrobust(a: *const pthread_mutexattr_t, robust: *mut c_int) -> c_int;
  extern "C" fn pthread_mutexattr_gettype(a: *const pthread_mutexattr_t, type_: *mut c_int) -> c_int;
  extern "C" fn pthread_rwlockattr_getpshared(a: *const pthread_rwlockattr_t, pshared: *mut c_int) -> c_int;
  // 所有函数与 C ABI 兼容，仅执行纯读取操作
Internal Interface:
  (无内部接口 — 所有函数直接映射到对外导出符号)
