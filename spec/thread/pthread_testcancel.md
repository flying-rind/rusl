# pthread_testcancel.c 规约

> musl libc 的线程显式取消点函数实现。在代码中显式插入一个取消点，允许挂起的取消请求在此处生效。

---

## 依赖图

```
pthread_testcancel (弱别名 -> __pthread_testcancel)
  ├─> __testcancel()  — hidden (见 pthread_cancel.c)
  │     ├─> __pthread_self()  — see internal/pthread_impl.h
  │     └─> __cancel()        — hidden (见 pthread_cancel.c)
  └─> dummy()         — static (同文件, 空函数)

注: __testcancel 定义在 pthread_cancel.c 中，本文件仅声明引用。
```

---

## 内部函数规约

### 1. dummy (static)

```c
static void dummy(void);
```

[Visibility]: Internal (不导出) — static 空函数，仅在启动早期/单线程环境中作为 `__testcancel` 的弱别名占位

#### Intent

在 `pthread_cancel` 首次调用并初始化取消机制之前，`__testcancel` 弱别名指向该占位空函数，避免调用未初始化的取消逻辑。

#### 后置条件

- 无操作（No-op）

---

### 2. __pthread_testcancel (hidden)

```c
void __pthread_testcancel(void);
```

[Visibility]: Internal (不导出) — `hidden` 可见性，`pthread_testcancel` 的主实现

#### Intent

为 `pthread_testcancel` 提供实际实现。

#### 系统算法

```
__pthread_testcancel():
  1. __testcancel()  // 检查 cancel 标志, 若设置则执行取消
```

---

## 对外导出函数规约

### 3. pthread_testcancel

```c
void pthread_testcancel(void);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)，是 `__pthread_testcancel` 的弱别名

#### Intent

在调用点显式检查是否有挂起的取消请求。如果当前线程已被其他线程通过 `pthread_cancel` 请求取消，且取消未被禁用（`canceldisable == PTHREAD_CANCEL_ENABLE`），则在此处执行取消并终止线程。

#### 前置条件

- 调用线程的取消状态为启用（`canceldisable != PTHREAD_CANCEL_DISABLE`）

#### 后置条件

- Case 1（存在挂起的取消请求）：调用 `__cancel()` 执行取消，线程终止（不返回）
- Case 2（无挂起的取消请求或取消被禁用）：立即返回，线程继续正常执行

#### 系统算法

```
pthread_testcancel():
  1. 等同于 __pthread_testcancel()
     = __testcancel()
     = if self->cancel && !self->canceldisable: __cancel()
```

#### 不变量

- 取消检查是幂等的：多次调用结果相同（除非状态在调用间被修改）

#### 依赖

- `__testcancel()` — 取消检查核心（定义在 `pthread_cancel.c`，见该文件 spec）
- `__pthread_self()` — 获取当前 `struct pthread *`（定义在 `internal/pthread_impl.h`）
- `__cancel()` — 执行实际的取消退出（定义在 `pthread_cancel.c`）
