# pthread_setcanceltype.c 规约

> musl libc 的线程取消类型设置函数实现。设置当前线程的取消类型为延迟取消或异步取消。

---

## 依赖图

```
pthread_setcanceltype
  ├─> __pthread_self()    — see internal/pthread_impl.h (宏)
  └─> pthread_testcancel() — see pthread_testcancel.c
```

---

## 对外导出函数规约

### 1. pthread_setcanceltype

```c
int pthread_setcanceltype(int new, int *old);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

设置调用线程的取消类型。延迟取消（默认）仅在取消点进行检查；异步取消可在任意时刻生效。切换到异步取消时，若有挂起的取消请求则立即生效。

#### 前置条件

- `new` 必须为 `PTHREAD_CANCEL_DEFERRED` (0) 或 `PTHREAD_CANCEL_ASYNCHRONOUS` (1)
- `old` 可为 NULL

#### 后置条件

- Case 1（`new > 1U`，无效值）：返回 `EINVAL`，`self->cancelasync` 不变
- Case 2（有效值）：若 `old != NULL`，`*old = self->cancelasync`。`self->cancelasync = new`。若 `new != 0`（切换到异步），调用 `pthread_testcancel()` 立即检查是否有挂起的取消请求。返回 0。

#### 系统算法

```
pthread_setcanceltype(new, old):
  1. self = __pthread_self()
  2. if new > 1U: return EINVAL     // 无符号比较: 0 有效, >1 无效
  3. if old: *old = self->cancelasync
  4. self->cancelasync = new
  5. if new: pthread_testcancel()   // 切换到异步取消时立即检查
  6. return 0
```

#### 不变量

- `cancelasync` 字段（`unsigned char`）仅在值 0（DEFERRED）和 1（ASYNCHRONOUS）之间切换
- 取消类型不影响取消请求的存在性，仅影响生效时机

#### 依赖

- `__pthread_self()` — 获取当前 `struct pthread *`（宏，见 `internal/pthread_impl.h`）
- `pthread_testcancel()` — 取消点检查（见 `pthread_testcancel.c`）
- `EINVAL` — 错误码（来自 `<errno.h>`）
- `PTHREAD_CANCEL_DEFERRED` / `PTHREAD_CANCEL_ASYNCHRONOUS` — 常量（见 `<pthread.h>`）
