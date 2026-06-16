# pthread_getschedparam — Rust 接口归约

> rusl 获取指定线程的调度策略和优先级。通过内核系统调用读取线程的实际调度参数，需要持有目标线程的 `killlock` 以确保 TID 有效性。

---

## 原始 C 对外导出接口

```c
// 用户可见符号
int pthread_getschedparam(pthread_t t, int *restrict policy, struct sched_param *restrict param);
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
extern "C" fn pthread_getschedparam(
    t: *mut core::ffi::c_void,
    policy: *mut core::ffi::c_int,
    param: *mut SchedParam,
) -> core::ffi::c_int;
```

---

## 意图

获取目标线程 `t` 当前的调度策略（`SCHED_FIFO`/`SCHED_RR`/`SCHED_OTHER` 等）和调度参数（主要是优先级 `sched_priority`）。通过内核系统调用 `SYS_sched_getparam` 和 `SYS_sched_getscheduler` 从内核读取。

---

## 前置条件

- `t` 不为 null，指向有效的 `struct pthread`
- `policy` 不为 null，指向可写入 `c_int` 的内存
- `param` 不为 null，指向可写入 `SchedParam` 的内存
- `policy` 和 `param` 不重叠（restrict 约束）

---

## 后置条件

- Case 1 成功（`t->tid != 0`）：
  - 通过 `SYS_sched_getparam` 获取调度参数，写入 `*param`
  - 通过 `SYS_sched_getscheduler` 获取调度策略，写入 `*policy`
  - 返回 0
- Case 2 成功（`t->tid != 0`，但 `sched_getparam` 系统调用失败）：
  - 返回负的内核错误码（`-errno`）
  - `*policy` 未被修改
- Case 3 失败（`t->tid == 0`，线程已退出）：
  - 返回 `ESRCH`
  - `*policy` 和 `*param` 未修改

---

## 不变量

- `killlock` 在访问 `t->tid` 和发起内核调度系统调用期间被持有，保证 TID 有效性

---

## 算法

```
pthread_getschedparam(t, policy, param):
  1. // 阻塞应用层信号以保障 AS-safety
     block_app_sigs(&old_set)

  2. // 获取 killlock 确保 TID 有效
     lock(t.killlock)

  3. if t.tid == 0:
       // 线程已退出
       r = ESRCH
     else:
       r = -syscall(SYS_sched_getparam, t.tid, param)
       if r == 0:
         *policy = syscall(SYS_sched_getscheduler, t.tid) as c_int

  4. unlock(t.killlock)
  5. restore_sigs(&old_set)
  6. return r
```

Rust 内部可使用 RAII 守卫管理 `killlock` 和信号阻塞的配对释放：

```rust
pub(crate) fn getschedparam_impl(t: &Pthread, policy: &mut c_int, param: &mut SchedParam) -> c_int {
    let _sig_guard = block_app_sigs();       // RAII: Drop 时自动恢复信号
    let _kl_guard = t.killlock.lock();       // RAII: Drop 时自动释放锁
    if t.tid == 0 {
        return ESRCH;
    }
    // ... 系统调用
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::c_int                                        // 依赖1: C ABI 基本类型
  __syscall(SYS_sched_getparam, ...)   (外部模块 syscall)  // 依赖2: 内核调度参数读取
  __syscall(SYS_sched_getscheduler, ...) (外部模块 syscall) // 依赖3: 内核调度策略读取
  __block_app_sigs / __restore_sigs    (外部模块 thread)   // 依赖4: 信号掩码管理
  killlock (lock.h)                                      // 依赖5: 线程 killlock 自旋锁

Predefined Macros/Constants:
  ESRCH, SYS_sched_getparam, SYS_sched_getscheduler

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_getschedparam(t: *mut core::ffi::c_void, policy: *mut core::ffi::c_int,
                                      param: *mut SchedParam) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_getschedparam 符号
