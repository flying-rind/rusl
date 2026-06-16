# pthread_setschedprio — Rust 接口归约

> rusl 设置指定线程的调度优先级。通过内核 `sched_setparam` 系统调用修改优先级（不改变调度策略），需要持有目标线程的 `killlock` 以确保 TID 有效性。

---

## 原始 C 对外导出接口

```c
// 用户可见符号
int pthread_setschedprio(pthread_t t, int prio);
```

---

## Rust 外部 ABI 接口

```rust
// 用户可见符号 — 与 C ABI 兼容
extern "C" fn pthread_setschedprio(
    t: *mut core::ffi::c_void,
    prio: core::ffi::c_int,
) -> core::ffi::c_int;
```

---

## 意图

修改目标线程 `t` 的调度优先级，但不改变其调度策略。通过内核系统调用 `SYS_sched_setparam` 实现。注意：C 实现中将 `&prio` 作为 `struct sched_param *` 传递，因为 `struct sched_param` 的第一个（也是唯一有效的）字段是 `sched_priority`。

---

## 前置条件

- `t` 不为 null，指向有效的 `struct pthread`
- `prio` 在当前调度策略的合法优先级范围内

---

## 后置条件

- Case 1 成功（`t->tid != 0` 且内核操作成功）：
  - 目标线程的调度优先级被设置为 `prio`
  - 调度策略不变
  - 返回 0
- Case 2 失败（`t->tid == 0`，线程已退出）：
  - 返回 `ESRCH`
- Case 3 失败（`t->tid != 0` 但内核操作失败）：
  - 返回负的内核错误码（例如 `-EINVAL` 优先级无效，`-EPERM` 权限不足）

---

## 不变量

- `killlock` 在检查 `t->tid` 和发起调度系统调用期间被持有

---

## 算法

```
pthread_setschedprio(t, prio):
  1. // 阻塞应用层信号以保障 AS-safety
     block_app_sigs(&old_set)

  2. // 获取 killlock 确保 TID 有效
     lock(t.killlock)

  3. if t.tid == 0:
       r = ESRCH
     else:
       // sched_setparam 仅使用 struct sched_param 的第一个字段 (sched_priority)
       // 可将 &prio 视为 SchedParam* 传递
       r = -syscall(SYS_sched_setparam, t.tid, &prio)

  4. unlock(t.killlock)
  5. restore_sigs(&old_set)
  6. return r
```

Rust 内部可在栈上构造临时 `SchedParam { sched_priority: prio }` 传递给系统调用，无需依赖 C 的内存布局技巧。

```rust
// 内部实现可以更清晰地构造 SchedParam
pub(crate) fn setschedprio_impl(t: &Pthread, prio: c_int) -> c_int {
    let _sig_guard = block_app_sigs();
    let _kl_guard = t.killlock.lock();
    if t.tid == 0 {
        return ESRCH;
    }
    let param = SchedParam { sched_priority: prio };
    -syscall(SYS_sched_setparam, t.tid, &param)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::c_int                                        // 依赖1: C ABI 基本类型
  __syscall(SYS_sched_setparam, ...)    (外部模块 syscall)  // 依赖2: 内核调度参数设置
  __block_app_sigs / __restore_sigs    (外部模块 thread)   // 依赖3: 信号掩码管理
  killlock (lock.h)                                      // 依赖4: 线程 killlock 自旋锁

Predefined Macros/Constants:
  ESRCH, SYS_sched_setparam

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_setschedprio(t: *mut core::ffi::c_void, prio: core::ffi::c_int) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_setschedprio 符号
