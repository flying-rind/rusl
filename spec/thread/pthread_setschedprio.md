# pthread_setschedprio.c 规约

> musl libc 设置指定线程的调度优先级。通过内核 `sched_setparam` 系统调用修改优先级（不改变调度策略），需要持有目标线程的 `killlock` 以确保 TID 有效性。

---

## 依赖图

```
pthread_setschedprio
  ├── LOCK(t->killlock)      (see lock.h)
  ├── UNLOCK(t->killlock)    (see lock.h)
  ├── __block_app_sigs       (外部模块: thread/sigaction)
  ├── __restore_sigs         (外部模块: thread/sigaction)
  └── __syscall              (外部模块: syscall)
```

---

## 函数规约

### 1. pthread_setschedprio

```c
int pthread_setschedprio(pthread_t t, int prio);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

修改目标线程 `t` 的调度优先级，但不改变其调度策略。通过内核系统调用 `SYS_sched_setparam` 实现。

#### 前置条件

- `t != NULL` 且指向有效的 `struct pthread`
- `prio` 在当前调度策略的合法优先级范围内

#### 后置条件

- Case 1 成功（`t->tid != 0` 且内核操作成功）：
  - 目标线程的调度优先级被设置为 `prio`
  - 调度策略不变
  - 返回 0
- Case 2 失败（`t->tid == 0`，线程已退出）：
  - 返回 `ESRCH`
- Case 3 失败（`t->tid != 0` 但内核操作失败）：
  - 返回负的内核错误码（例如 `-EINVAL` 优先级无效，`-EPERM` 权限不足）

#### 系统算法

```
pthread_setschedprio(t, prio):
  1. __block_app_sigs(&set)
     // 阻塞应用层信号以保障 AS-safety
  2. LOCK(t->killlock)
     // 获取 killlock 以确保在访问期间 TID 保持有效
  3. r = (t->tid == 0) ? ESRCH
       : -__syscall(SYS_sched_setparam, t->tid, &prio)
     // 注意: &prio 被传递为 sched_param*：
     // sched_setparam 仅使用第一个 int 字段 (sched_priority)
  4. UNLOCK(t->killlock)
  5. __restore_sigs(&set)
  6. return r
```

#### 不变量

- `killlock` 在检查 `t->tid` 和发起调度系统调用期间被持有

#### 依赖

| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `LOCK(t->killlock)` / `UNLOCK(t->killlock)` | `lock.h` (内部) | 自旋锁 |
| `__block_app_sigs` | 外部模块 thread | 阻塞应用层信号 |
| `__restore_sigs` | 外部模块 thread | 恢复信号掩码 |
| `__syscall(SYS_sched_setparam, ...)` | 外部模块 syscall | 内核调度参数设置 |
| `pthread_impl.h` | 内部头文件 | `struct pthread` 定义 |
