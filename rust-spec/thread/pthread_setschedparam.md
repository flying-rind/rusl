# pthread_setschedparam — Rust 接口归约

> rusl 设置指定线程的调度策略和优先级。通过内核 `sched_setscheduler` 系统调用修改调度属性，需要持有目标线程的 `killlock` 以确保 TID 有效性。

---

## 原始 C 对外导出接口

```c
// 用户可见符号
int pthread_setschedparam(pthread_t t, int policy, const struct sched_param *param);
```

---

## Rust 外部 ABI 接口

```rust
// SchedParam — 与 C struct sched_param 内存布局兼容
#[repr(C)]
struct SchedParam {
    sched_priority: core::ffi::c_int,
}

// 用户可见符号 — 与 C ABI 兼容
extern "C" fn pthread_setschedparam(
    t: *mut core::ffi::c_void,
    policy: core::ffi::c_int,
    param: *const SchedParam,
) -> core::ffi::c_int;
```

---

## 意图

修改目标线程 `t` 的调度策略和调度参数（优先级）。通过内核系统调用 `SYS_sched_setscheduler` 原子地同时设置策略和参数。

---

## 前置条件

- `t` 不为 null，指向有效的 `struct pthread`
- `policy` 为合法的调度策略（`SCHED_OTHER`/`SCHED_FIFO`/`SCHED_RR`/`SCHED_BATCH`/`SCHED_IDLE`/`SCHED_DEADLINE`）
- `param` 不为 null，指向有效的 `SchedParam`，其中 `sched_priority` 在策略允许的优先级范围内

---

## 后置条件

- Case 1 成功（`t->tid != 0` 且内核操作成功）：
  - 目标线程的调度策略被设置为 `policy`
  - 目标线程的调度优先级被设置为 `param.sched_priority`
  - 返回 0
- Case 2 失败（`t->tid == 0`，线程已退出）：
  - 返回 `ESRCH`
- Case 3 失败（`t->tid != 0` 但内核操作失败）：
  - 返回负的内核错误码（例如 `-EINVAL` 如果策略/优先级无效，`-EPERM` 权限不足）

---

## 不变量

- `killlock` 在检查 `t->tid` 和发起调度系统调用期间被持有，保证内核 TID 在此期间不被回收

---

## 算法

```
pthread_setschedparam(t, policy, param):
  1. // 阻塞应用层信号以保障 AS-safety
     block_app_sigs(&old_set)

  2. // 获取 killlock 确保 TID 有效
     lock(t.killlock)

  3. if t.tid == 0:
       r = ESRCH
     else:
       r = -syscall(SYS_sched_setscheduler, t.tid, policy, param)

  4. unlock(t.killlock)
  5. restore_sigs(&old_set)
  6. return r
```

Rust 内部可使用 RAII 守卫模式，与 `pthread_getschedparam` 共享相同的锁管理基础设施。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::c_int                                        // 依赖1: C ABI 基本类型
  __syscall(SYS_sched_setscheduler, ...) (外部模块 syscall) // 依赖2: 内核调度设置
  __block_app_sigs / __restore_sigs    (外部模块 thread)   // 依赖3: 信号掩码管理
  killlock (lock.h)                                      // 依赖4: 线程 killlock 自旋锁

Predefined Macros/Constants:
  ESRCH, SYS_sched_setscheduler

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_setschedparam(t: *mut core::ffi::c_void, policy: core::ffi::c_int,
                                      param: *const SchedParam) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_setschedparam 符号
