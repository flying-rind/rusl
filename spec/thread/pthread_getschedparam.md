# pthread_getschedparam.c 规约

> musl libc 获取指定线程的调度策略和优先级。通过内核系统调用读取线程的实际调度参数，需要持有目标线程的 `killlock` 以确保 TID 有效性。

---

## 依赖图

```
pthread_getschedparam
  ├── LOCK(t->killlock)      (see lock.h)
  ├── UNLOCK(t->killlock)    (see lock.h)
  ├── __block_app_sigs       (外部模块: thread/sigaction)
  ├── __restore_sigs         (外部模块: thread/sigaction)
  └── __syscall              (外部模块: syscall)
```

---

## 函数规约

### 1. pthread_getschedparam

```c
int pthread_getschedparam(pthread_t t, int *restrict policy, struct sched_param *restrict param);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

获取目标线程 `t` 当前的调度策略（SCHED_FIFO/SCHED_RR/SCHED_OTHER 等）和调度参数（主要是优先级 `sched_priority`）。通过内核系统调用 `SYS_sched_getparam` 和 `SYS_sched_getscheduler` 从内核读取。

#### 前置条件

- `t != NULL` 且指向有效的 `struct pthread`
- `policy != NULL`，指向可写入 `int` 的内存
- `param != NULL`，指向可写入 `struct sched_param` 的内存
- `policy` 和 `param` 不重叠（restrict 约束）

#### 后置条件

- Case 1 成功（`t->tid != 0`）：
  - 通过 `SYS_sched_getparam` 获取调度参数，写入 `*param`
  - 通过 `SYS_sched_getscheduler` 获取调度策略，写入 `*policy`
  - 返回 0
- Case 2 成功（`t->tid != 0`，但 `sched_getparam` 失败）：
  - 返回负的内核错误码（`-errno`）
  - `*policy` 未被修改
- Case 3 失败（`t->tid == 0`，线程已退出）：
  - 返回 `ESRCH`
  - `*policy` 和 `*param` 未修改

#### 系统算法

```
pthread_getschedparam(t, policy, param):
  1. __block_app_sigs(&set)
     // 阻塞应用层信号以保障 AS-safety
  2. LOCK(t->killlock)
     // 获取 killlock 以确保在访问期间 TID 保持有效
  3. if t->tid == 0:
       // 线程已退出，TID 无效
       r = ESRCH
     else:
       r = -__syscall(SYS_sched_getparam, t->tid, param)
       if r == 0:
         *policy = __syscall(SYS_sched_getscheduler, t->tid)
  4. UNLOCK(t->killlock)
  5. __restore_sigs(&set)
  6. return r
```

#### 不变量

- `killlock` 在访问 `t->tid` 和发起内核调度系统调用期间被持有，保证 TID 有效性

#### 依赖

| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `LOCK(t->killlock)` / `UNLOCK(t->killlock)` | `lock.h` (内部) | 自旋锁，转换为 `__lock`/`__unlock` |
| `__block_app_sigs` | 外部模块 thread | 阻塞所有应用层信号，保存旧掩码到 `sigset_t*` |
| `__restore_sigs` | 外部模块 thread | 恢复信号掩码 |
| `__syscall(SYS_sched_getparam, ...)` | 外部模块 syscall | 内核调度参数读取 |
| `__syscall(SYS_sched_getscheduler, ...)` | 外部模块 syscall | 内核调度策略读取 |
| `pthread_impl.h` | 内部头文件 | `struct pthread` 定义，`tid` 字段 |
