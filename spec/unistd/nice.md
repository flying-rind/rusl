# nice.c 规约

> musl libc POSIX 进程调度优先级调整函数。`nice` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
nice (Public)
  ├── getpriority(PRIO_PROCESS, 0) — 获取当前进程调度优先级
  │     └── PRIO_PROCESS — 进程优先级类型宏 (<sys/resource.h>)
  ├── setpriority(PRIO_PROCESS, 0, prio) — 设置当前进程调度优先级
  │     └── errno (EACCES → EPERM 映射)
  └── NZERO — 默认优先级常量 (<sys/resource.h>)
```

---

## 函数规约

### nice

```c
int nice(int inc);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数（X/Open 扩展，SVr4, 4.3BSD），用户程序可直接调用

#### Intent

将调用进程的调度优先级（nice 值）增加 `inc`。nice 值范围通常为 `[-NZERO, NZERO-1]`（即 [-20, 19]），较小的值（负值）意味着较高的调度优先级。只有具有适当权限的进程（通常是 root）才能将 nice 值降低（提高优先级）。

该实现使用 `getpriority`/`setpriority` 而非已弃用的 `SYS_nice` 系统调用。包含一个优化：当 `inc` 超出范围时跳过 `getpriority` 查询，直接进行边界裁剪。

#### 前置条件

- `inc`: 要增加到当前 nice 值的增量（可以为负）

#### 后置条件

- **Case 1 成功（有权限）**
  - 进程的 nice 值增加 `inc`，并被裁剪到 `[-NZERO, NZERO-1]` 范围内
  - 返回新的 nice 值（可能不同于 `old_nice + inc`，因为经过了边界裁剪）

- **Case 2 成功（无权限、试图降低 nice 值为负）**
  - nice 值不变
  - 返回新的 nice 值（即原始值，因为 `setpriority` 允许非特权进程提高 nice 值但忽略降低请求）

- **Case 3 错误（权限不足无法调用 setpriority）**
  - nice 值不变
  - 返回 -1
  - `errno` 设置为 `EPERM`（musl 将内核返回的 `EACCES` 映射为 `EPERM`，以符合 POSIX 规范）

- **注意**：由于 -1 也是合法的 nice 返回值（nice 值 -1 表示优先级为 -1），调用者应通过"调用前将 `errno` 设为 0，调用后检查 `errno != 0`"的模式来区分成功和失败。

#### 系统算法

```
nice(inc):
  prio = inc                      // 1. 初始化目标增量为 inc
  if inc > -2*NZERO && inc < 2*NZERO:  // 2. 仅当增量在合理范围内才查询旧值
    prio += getpriority(PRIO_PROCESS, 0)  //   避免溢出和无效系统调用
  if prio > NZERO-1: prio = NZERO-1       // 3. 上界裁剪
  if prio < -NZERO:  prio = -NZERO        // 4. 下界裁剪
  if setpriority(PRIO_PROCESS, 0, prio):  // 5. 设置新优先级
    if errno == EACCES: errno = EPERM     // 6. 映射错误码
    return -1
  return prio                              // 7. 返回新 nice 值
```

#### 依赖

- `getpriority(int, id_t)` — 来自 `<sys/resource.h>`，获取进程/进程组/用户的调度优先级
- `setpriority(int, id_t, int)` — 来自 `<sys/resource.h>`，设置进程/进程组/用户的调度优先级
- `PRIO_PROCESS` — 来自 `<sys/resource.h>`，指定操作为进程优先级（值通常为 0）
- `NZERO` — 来自 `<sys/resource.h>`（通过 `<limits.h>`），默认 nice 值（通常为 20）
- `EACCES`, `EPERM` — 来自 `<errno.h>`，错误码
