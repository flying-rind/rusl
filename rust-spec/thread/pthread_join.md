# pthread_join — Rust 接口归约

> rusl 等待目标线程结束并获取其返回值。核心实现为 `__pthread_timedjoin_np`，通过 futex 等待 `detach_state` 变为 `DT_EXITED`。`__pthread_join` 是其超时为 null（无限等待）的包装。

---

## 原始 C 对外导出接口

```c
// 用户可见符号 (weak_alias)
int pthread_join(pthread_t t, void **res);
int pthread_timedjoin_np(pthread_t t, void **res, const struct timespec *at);
int pthread_tryjoin_np(pthread_t t, void **res);

// musl 内部跨模块调用符号
int __pthread_join(pthread_t t, void **res);
int __pthread_timedjoin_np(pthread_t t, void **res, const struct timespec *at);
int __pthread_tryjoin_np(pthread_t t, void **res);
```

---

## Rust 外部 ABI 接口

```rust
// 用户可见符号 — 与 C ABI 兼容
extern "C" fn pthread_join(
    t: *mut core::ffi::c_void,
    res: *mut *mut core::ffi::c_void,
) -> core::ffi::c_int;

extern "C" fn pthread_timedjoin_np(
    t: *mut core::ffi::c_void,
    res: *mut *mut core::ffi::c_void,
    at: *const Timespec,
) -> core::ffi::c_int;

extern "C" fn pthread_tryjoin_np(
    t: *mut core::ffi::c_void,
    res: *mut *mut core::ffi::c_void,
) -> core::ffi::c_int;

// musl 内部跨模块调用符号 — 与 C ABI 兼容
extern "C" fn __pthread_join(
    t: *mut core::ffi::c_void,
    res: *mut *mut core::ffi::c_void,
) -> core::ffi::c_int;

extern "C" fn __pthread_timedjoin_np(
    t: *mut core::ffi::c_void,
    res: *mut *mut core::ffi::c_void,
    at: *const Timespec,
) -> core::ffi::c_int;

extern "C" fn __pthread_tryjoin_np(
    t: *mut core::ffi::c_void,
    res: *mut *mut core::ffi::c_void,
) -> core::ffi::c_int;
```

---

## 内部类型定义

### Timespec（repr(C)）

```rust
#[repr(C)]
struct Timespec {
    tv_sec: i64,   // 秒
    tv_nsec: i64,  // 纳秒
}
```

---

## 意图

### pthread_join / __pthread_join
等待线程 `t` 终止并获取其返回值。是 `__pthread_timedjoin_np` 的超时为 null 的包装（无限等待）。

### pthread_timedjoin_np / __pthread_timedjoin_np
等待目标线程 `t` 的 `detach_state` 变为 `DT_EXITED`，支持超时。成功后将线程返回值写入 `*res` 并释放线程的映射资源。实现了取消安全：在等待前测试取消点，等待期间禁用取消以避免不一致状态。

### pthread_tryjoin_np / __pthread_tryjoin_np
尝试等待线程 `t` 终止但不阻塞。如果线程仍为 `DT_JOINABLE`，立即返回 `EBUSY`。

---

## 前置条件

### __pthread_timedjoin_np
- `t` 不为 null，指向有效的 `struct pthread`
- 若 `t->detach_state >= DT_DETACHED` 且为 `DT_JOINABLE`，对于已分离线程调用是未定义行为（触发 crash）
- `at` 可为 null 表示无限等待，否则指向基于 `CLOCK_REALTIME` 的绝对超时时间

### __pthread_join
- 与 `__pthread_timedjoin_np(t, res, null())` 相同
- 永不返回 `ETIMEDOUT`（无限等待）

### __pthread_tryjoin_np
- 与 `__pthread_join` 相同
- 额外检查当前分离状态

---

## 后置条件

### __pthread_timedjoin_np
- Case 1 成功（`detach_state` 变为 `DT_EXITED`）：
  - `__tl_sync(t)` 等待线程列表锁释放
  - 若 `res` 不为 null：`*res = t->result`（线程退出值）
  - 若 `t->map_base` 不为 null：`__munmap` 释放线程栈和 guard 映射
  - 返回 0
- Case 2 超时：返回 `ETIMEDOUT`
- Case 3 参数无效：返回 `EINVAL`
- Case 4 检测到取消请求：线程被取消（不返回）
- Case 5 目标线程已分离且非 `DT_JOINABLE`：进程终止（crash）

### __pthread_join
- 与 `__pthread_timedjoin_np` 相同，但不会返回 `ETIMEDOUT`

### __pthread_tryjoin_np
- Case 1（`detach_state == DT_JOINABLE`）：返回 `EBUSY`
- Case 2（已退出或正在退出）：行为等同于 `__pthread_join(t, res)`

---

## 不变量

- `t->detach_state` 在等待期间由 futex 保护
- 在关键等待阶段取消被禁用以保证状态一致性

---

## 算法

### __pthread_timedjoin_np 内部实现

```
__pthread_timedjoin_np_impl(t, res, at):
  1. // 取消点: 若存在待处理的取消请求，此处终止
     pthread_testcancel()

  2. // 进入取消禁用区域
     cs = pthread_setcancelstate(PTHREAD_CANCEL_DISABLE)
     if cs == PTHREAD_CANCEL_ENABLE:
         // 恢复以允许取消
         pthread_setcancelstate(PTHREAD_CANCEL_ENABLE)

  3. // 循环等待 detach_state 变为 DT_EXITED (0)
     loop:
       state = t.detach_state.load(Acquire)
       if state == 0:  break  // DT_EXITED
       if state >= DT_DETACHED:
           abort()  // 非法操作
       r = timedwait_cp(&t.detach_state, state, CLOCK_REALTIME, at)
       if r == ETIMEDOUT || r == EINVAL:
           return r

  4. // 恢复取消状态
     pthread_setcancelstate(cs)

  5. // 等待线程列表同步完成
     __tl_sync(t)

  6. // 写入返回值
     if res != null:  *res = t.result

  7. // 释放线程栈映射
     if t.map_base != null:  munmap(t.map_base, t.map_size)

  8. return 0
```

### __pthread_join 内部实现

```
__pthread_join_impl(t, res):
  return __pthread_timedjoin_np_impl(t, res, null())
```

### __pthread_tryjoin_np 内部实现

```
__pthread_tryjoin_np_impl(t, res):
  if t.detach_state.load(Acquire) == DT_JOINABLE:
      return EBUSY
  return __pthread_join_impl(t, res)
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::sync::atomic::AtomicI32                            // 依赖1: 原子加载 detach_state
  core::ffi::{c_int, c_void}                                // 依赖2: C ABI 基本类型
  __pthread_testcancel    (外部模块 thread/cancel)         // 依赖3: 取消点测试
  __pthread_setcancelstate (外部模块 thread/cancel)        // 依赖4: 设置/恢复取消状态
  __timedwait_cp           (外部模块 thread)               // 依赖5: 带取消点的 futex 超时等待
  __tl_sync                (本模块 pthread_create)         // 依赖6: 等待线程列表同步
  __munmap                 (外部模块 mmap)                 // 依赖7: 释放栈映射
  DetachState 枚举定义     (本模块 pthread_create)          // 依赖8: DT_JOINABLE/DT_DETACHED/DT_EXITING/DT_EXITED

Predefined Types:
  struct pthread (pthread_impl.h)                           // 线程结构体定义

Predefined Macros/Constants:
  CLOCK_REALTIME, ETIMEDOUT, EINVAL, EBUSY, PTHREAD_CANCEL_DISABLE, PTHREAD_CANCEL_ENABLE

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_join(t: *mut core::ffi::c_void, res: *mut *mut core::ffi::c_void) -> core::ffi::c_int;
  extern "C" fn pthread_timedjoin_np(t: *mut core::ffi::c_void, res: *mut *mut core::ffi::c_void,
                                     at: *const Timespec) -> core::ffi::c_int;
  extern "C" fn pthread_tryjoin_np(t: *mut core::ffi::c_void, res: *mut *mut core::ffi::c_void) -> core::ffi::c_int;
  extern "C" fn __pthread_join(t: *mut core::ffi::c_void, res: *mut *mut core::ffi::c_void) -> core::ffi::c_int;
  extern "C" fn __pthread_timedjoin_np(t: *mut core::ffi::c_void, res: *mut *mut core::ffi::c_void,
                                       at: *const Timespec) -> core::ffi::c_int;
  extern "C" fn __pthread_tryjoin_np(t: *mut core::ffi::c_void, res: *mut *mut core::ffi::c_void) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的上述符号
