# pthread_setschedparam.c 规约

> musl libc 设置指定线程的调度策略和优先级。通过内核 `sched_setscheduler` 系统调用修改调度属性，需要持有目标线程的 `killlock` 以确保 TID 有效性。

---

## 依赖图

```
pthread_setschedparam
  ├── LOCK(t->killlock)      (see lock.h)
  ├── UNLOCK(t->killlock)    (see lock.h)
  ├── __block_app_sigs       (外部模块: thread/sigaction)
  ├── __restore_sigs         (外部模块: thread/sigaction)
  └── __syscall              (外部模块: syscall)
```

---

## 函数规约

### 1. pthread_setschedparam

```c
int pthread_setschedparam(pthread_t t, int policy, const struct sched_param *param);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

修改目标线程 `t` 的调度策略和调度参数（优先级）。通过内核系统调用 `SYS_sched_setscheduler` 原子地同时设置策略和参数。

#### 前置条件

- `t != NULL` 且指向有效的 `struct pthread`
- `policy` 为合法的调度策略（`SCHED_OTHER`/`SCHED_FIFO`/`SCHED_RR`/`SCHED_BATCH`/`SCHED_IDLE`/`SCHED_DEADLINE`）
- `param != NULL`，指向有效的 `struct sched_param`，其中 `sched_priority` 在策略允许的优先级范围内

#### 后置条件

- Case 1 成功（`t->tid != 0` 且内核操作成功）：
  - 目标线程的调度策略被设置为 `policy`
  - 目标线程的调度优先级被设置为 `param->sched_priority`
  - 返回 0
- Case 2 失败（`t->tid == 0`，线程已退出）：
  - 返回 `ESRCH`
- Case 3 失败（`t->tid != 0` 但内核操作失败）：
  - 返回负的内核错误码（例如 `-EINVAL` 如果策略/优先级无效，`-EPERM` 权限不足）

#### 系统算法

```
pthread_setschedparam(t, policy, param):
  1. __block_app_sigs(&set)
     // 阻塞应用层信号以保障 AS-safety
  2. LOCK(t->killlock)
     // 获取 killlock 以确保在访问期间 TID 保持有效
  3. r = (t->tid == 0) ? ESRCH
       : -__syscall(SYS_sched_setscheduler, t->tid, policy, param)
  4. UNLOCK(t->killlock)
  5. __restore_sigs(&set)
  6. return r
```

#### 不变量

- `killlock` 在检查 `t->tid` 和发起调度系统调用期间被持有，保证内核 TID 在此期间不被回收

#### 依赖

| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `LOCK(t->killlock)` / `UNLOCK(t->killlock)` | `lock.h` (内部) | 自旋锁 |
| `__block_app_sigs` | 外部模块 thread | 阻塞应用层信号 |
| `__restore_sigs` | 外部模块 thread | 恢复信号掩码 |
| `__syscall(SYS_sched_setscheduler, ...)` | 外部模块 syscall | 内核调度设置 |
| `pthread_impl.h` | 内部头文件 | `struct pthread` 定义 |
