# pthread_setcancelstate.c 规约

> musl libc 的线程取消状态设置函数实现。原子地设置/查询当前线程的取消启用/禁用状态。

---

## 依赖图

```
pthread_setcancelstate (弱别名 -> __pthread_setcancelstate)
  └─> __pthread_self()  — see internal/pthread_impl.h (宏)
```

---

## 内部函数规约

### 1. __pthread_setcancelstate (hidden)

```c
int __pthread_setcancelstate(int new, int *old);
```

[Visibility]: Internal (不导出) — `hidden` 可见性，`pthread_setcancelstate` 的主实现

#### Intent

设置调用线程的取消状态（启用或禁用），并可选择性地返回之前的状态。

#### 前置条件

- `new` 必须为 `PTHREAD_CANCEL_ENABLE` (0) 或 `PTHREAD_CANCEL_DISABLE` (1)
- `old` 可为 NULL（不关心旧状态）

#### 后置条件

- Case 1（`new > 2U`，无效值）：返回 `EINVAL`，`self->canceldisable` 不变
- Case 2（有效值）：若 `old != NULL`，`*old = self->canceldisable`（旧状态）。`self->canceldisable = new`。返回 0。

#### 系统算法

```
__pthread_setcancelstate(new, old):
  1. if new > 2U: return EINVAL     // 无符号比较: 0 和 1 通过
  2. self = __pthread_self()
  3. if old: *old = self->canceldisable
  4. self->canceldisable = new
  5. return 0
```

---

## 对外导出函数规约

### 2. pthread_setcancelstate

```c
int pthread_setcancelstate(int new, int *old);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)，是 `__pthread_setcancelstate` 的弱别名

#### Intent

原子地设置调用线程的取消状态。当取消被禁用时，来自 `pthread_cancel` 的取消请求将被暂缓（不会丢失），直到取消被重新启用。

#### 前置条件

- `new` 取值为 `PTHREAD_CANCEL_ENABLE` 或 `PTHREAD_CANCEL_DISABLE`

#### 后置条件

- 返回值 0 表示成功，`self->canceldisable` 被设为 `new`
- 返回值 `EINVAL` 表示 `new` 无效
- 若 `old != NULL`，`*old` 被设为先前的取消状态
- 当从禁用切换到启用时，之前挂起的取消请求将在下一个取消点生效

#### 系统算法

```
pthread_setcancelstate(new, old):
  1. 等同于 __pthread_setcancelstate(new, old)
```

#### 不变量

- `canceldisable` 字段（`unsigned char`）仅在值 0（ENABLE）和 1（DISABLE）之间切换
- 取消请求不被丢弃：`cancel` 标志和 `canceldisable` 是独立的

#### 依赖

- `__pthread_self()` — 获取当前 `struct pthread *`（宏，见 `internal/pthread_impl.h`）
- `EINVAL` — 错误码（来自 `<errno.h>`）
- `PTHREAD_CANCEL_ENABLE` / `PTHREAD_CANCEL_DISABLE` — 常量（见 `<pthread.h>`）
