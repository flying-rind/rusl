# pthread_detach.c 规约

> musl libc 将线程置为分离状态。通过原子 CAS 操作将 `detach_state` 从 `DT_JOINABLE` 改为 `DT_DETACHED`；若线程已在退出过程中，退化为阻塞式 join 以完成资源回收。

---

## 依赖图

```
pthread_detach (= __pthread_detach)
  ├── a_cas                     (外部模块: atomic)
  ├── __pthread_setcancelstate  (外部模块: thread)
  └── __pthread_join            (本模块: pthread_join.c)
```

---

## 内部类型与常量

### detach_state 枚举

定义于 `pthread_impl.h`：

```c
enum {
    DT_EXITED  = 0,   // 线程已结束
    DT_EXITING,       // = 1, 线程正在退出
    DT_JOINABLE,      // = 2, 线程可被 join
    DT_DETACHED,      // = 3, 线程已分离
};
```

---

## 函数规约

### 1. __pthread_detach

```c
static int __pthread_detach(pthread_t t);
```

[Visibility]: Internal — `static` 函数，不对外导出。通过 `weak_alias` 提供 `pthread_detach` 和 `thrd_detach` 对外符号。

#### Intent

将目标线程 `t` 的分离状态从 `DT_JOINABLE` 改为 `DT_DETACHED`。若 CAS 操作失败（线程已进入退出流程或已处于分离/退出状态），则通过 `__pthread_join` 阻塞等待线程退出以完成隐式的资源回收（分离的线程需要自行释放资源）。

#### 前置条件

- `t != NULL` 且指向有效的 `struct pthread`
- 调用者不能同时是目标线程自身（对自身 detach 的语义未定义）

#### 后置条件

- Case 1 成功（CAS 成功，`t->detach_state` 从 `DT_JOINABLE` 变为 `DT_DETACHED`）：
  - 目标线程被标记为分离状态
  - 当线程退出时，其资源（栈/映射内存等）将在 `__pthread_exit` 中自动回收
  - 返回 0
- Case 2 失败（`t->detach_state` 不为 `DT_JOINABLE`）：
  - 线程已在退出、已退出或已分离
  - 禁用取消，调用 `__pthread_join(t, 0)` 等待线程完成退出
  - 若线程已退出（`DT_EXITED`），`__pthread_join` 立即返回
  - 恢复取消状态
  - 返回 0

#### 系统算法

```
__pthread_detach(t):
  1. // 尝试原子地将 detach_state 从 DT_JOINABLE 改为 DT_DETACHED
     if a_cas(&t->detach_state, DT_JOINABLE, DT_DETACHED) != DT_JOINABLE:
       // CAS 失败：线程可能正在退出或已经分离
       // 需要 join 以确保资源被回收
       int cs
       __pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &cs)
       __pthread_join(t, 0)          // 阻塞等待线程退出完成
       __pthread_setcancelstate(cs, 0)
  2. return 0
  3. // 注意: 总是返回 0，即使 join 内部出错也忽略
```

#### 不变量

- `t->detach_state` 只能通过原子操作修改
- CAS 的三种可能旧值 `DT_JOINABLE`（成功）、`DT_EXITING`、`DT_DETACHED` 或 `DT_EXITED`（触发 join 路径）

#### 依赖

| 依赖项 | 来源 | 说明 |
|--------|------|------|
| `a_cas` | 外部模块 atomic | 原子比较交换 |
| `__pthread_setcancelstate` | 外部模块 thread | 设置取消状态 |
| `__pthread_join` | 本模块 (pthread_join.c) | 阻塞等待线程退出 |
| `pthread_impl.h` | 内部头文件 | `struct pthread`, detach_state 枚举 |
