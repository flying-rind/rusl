# pthread_join.c 规约

> musl libc 等待目标线程结束并获取其返回值。核心实现为 `__pthread_timedjoin_np`，通过 futex 等待 `detach_state` 变为 `DT_EXITED`。`__pthread_join` 是其超时为 NULL（无限等待）的包装。

---

## 依赖图

```
pthread_join (= __pthread_join)
  └── __pthread_timedjoin_np
        ├── __pthread_testcancel    (外部模块: thread/cancel)
        ├── __pthread_setcancelstate (外部模块: thread/cancel)
        ├── __timedwait_cp          (外部模块: thread)
        ├── __tl_sync               (本模块: pthread_create.c -> weak_alias)
        └── __munmap                (外部模块: mmap)

pthread_tryjoin_np (= __pthread_tryjoin_np)
  └── __pthread_join
```

---

## 函数规约

### 1. __pthread_timedjoin_np

```c
static int __pthread_timedjoin_np(pthread_t t, void **res, const struct timespec *at);
```

[Visibility]: Internal (不导出) — `static` 函数，通过 `weak_alias` 提供 `pthread_timedjoin_np` 对外符号（需 `_GNU_SOURCE`）。

#### Intent

等待目标线程 `t` 的 `detach_state` 变为 `DT_EXITED`（0），支持超时。成功后将线程返回值写入 `*res` 并释放线程的映射资源。实现了取消安全：在等待前测试取消点，等待期间禁用取消以避免不一致状态。

#### 前置条件

- `t != NULL` 且指向有效的 `struct pthread`
- `t->detach_state` 须为 `DT_JOINABLE`（=2）或 `DT_EXITING`（=1）或 `DT_DETACHED`（=3）
- 若 `t->detach_state >= DT_DETACHED` 且为 `DT_JOINABLE`，对于已分离（`>= DT_DETACHED`）的线程，行为是调用 `a_crash()`
- `at` 可为 NULL 表示无限等待，否则指向 `struct timespec` 指定绝对超时（基于 `CLOCK_REALTIME`）

#### 后置条件

- Case 1 成功（`detach_state` 变为 `DT_EXITED`）：
  - `__tl_sync(t)` 等待线程列表锁释放，确保线程完全从列表移除
  - 若 `res != NULL`：`*res = t->result`（线程的退出值）
  - 若 `t->map_base != NULL`：通过 `__munmap` 释放线程的栈和 guard 映射
  - 返回 0
- Case 2 超时：返回 `ETIMEDOUT`
- Case 3 参数无效：返回 `EINVAL`
- Case 4 检测到取消请求（在调用入口）：取消线程（不返回）
- Case 5 目标线程已分离（`detach_state >= DT_DETACHED`）但非 `DT_JOINABLE`：调用 `a_crash()`，终止进程

#### 系统算法

```
__pthread_timedjoin_np(t, res, at):
  1. __pthread_testcancel()
     // 取消点: 若存在待处理的取消请求，此处终止
  2. __pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &cs)
     // 进入取消禁用区域，防止在关键操作中被取消
  3. if cs == PTHREAD_CANCEL_ENABLE:
       // 该线程本来就在执行 join 期间被取消了，
       // 恢复为启用状态使其能被取消
       __pthread_setcancelstate(cs, 0)

  4. // 循环等待 detach_state 变为 DT_EXITED (0)
     while (state = t->detach_state) && r != ETIMEDOUT && r != EINVAL:
       if state >= DT_DETACHED:
         a_crash()  // 尝试 join 一个已分离的线程 → 未定义行为，crash
       r = __timedwait_cp(&t->detach_state, state,
                          CLOCK_REALTIME, at, 1)
       // __timedwait_cp 内部包含取消点

  5. // 恢复取消状态
     __pthread_setcancelstate(cs, 0)

  6. if r == ETIMEDOUT || r == EINVAL:
       return r

  7. // 等待线程列表同步完成
     __tl_sync(t)

  8. if res != NULL:  *res = t->result

  9. // 释放线程的栈映射
     if t->map_base:  __munmap(t->map_base, t->map_size)

  10. return 0
```

#### 不变量

- `t->detach_state` 在等待期间由 futex 保护
- 在步骤 2-4 之间取消被禁用，以保证状态一致性

---

### 2. __pthread_join

```c
int __pthread_join(pthread_t t, void **res);
```

[Visibility]: Internal (不导出) — 通过 `weak_alias` 对外提供 `pthread_join`。

#### Intent

等待线程 `t` 终止并获取其返回值。是 `__pthread_timedjoin_np` 的超时为 NULL 的包装。

#### 前置条件

- 等同于 `__pthread_timedjoin_np(t, res, 0)` 的前置条件
- `at` 为 NULL 表示无限等待（永不超时）

#### 后置条件

- 等同于 `__pthread_timedjoin_np` 的成功/失败条件
- 不会返回 `ETIMEDOUT`（因为无限等待）

#### 系统算法

```
__pthread_join(t, res):
  return __pthread_timedjoin_np(t, res, 0)
```

---

### 3. __pthread_tryjoin_np

```c
static int __pthread_tryjoin_np(pthread_t t, void **res);
```

[Visibility]: Internal (不导出) — 通过 `weak_alias` 对外提供 `pthread_tryjoin_np`（需 `_GNU_SOURCE`）。

#### Intent

尝试等待线程 `t` 终止，但不阻塞。如果线程仍为 `DT_JOINABLE` 状态，立即返回 `EBUSY`。

#### 前置条件

- 等同于 `__pthread_join` 的前置条件

#### 后置条件

- Case 1（线程仍为 `DT_JOINABLE`）：返回 `EBUSY`，不修改任何状态
- Case 2（线程已处于退出或已退出状态）：行为等同于 `__pthread_join(t, res)`

#### 系统算法

```
__pthread_tryjoin_np(t, res):
  return t->detach_state == DT_JOINABLE ? EBUSY
                                        : __pthread_join(t, res)
```

#### 依赖总结

| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `__pthread_testcancel` | 外部模块 thread | 取消点测试 |
| `__pthread_setcancelstate` | 外部模块 thread | 设置/恢复取消状态 |
| `__timedwait_cp` | 外部模块 thread | 带取消点的 futex 超时等待 |
| `__tl_sync` | 本模块 (pthread_create.c) | 等待线程列表同步 |
| `__munmap` | 外部模块 mmap | 释放栈映射 |
| `a_crash` | 外部模块 | 进程终止 |
| `pthread_impl.h` | 内部头文件 | `struct pthread`, detach_state 枚举 |
