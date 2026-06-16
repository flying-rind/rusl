# pthread_setspecific.c 规约

> musl libc 线程局部存储（TSD）值设置函数。

---

## 依赖图

```
pthread_setspecific
  └─> __pthread_self()  (see pthread_self.c spec)
```

---

## 函数规约

### 1. `pthread_setspecific`

```c
int pthread_setspecific(pthread_key_t k, const void *x);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

将调用线程中键 `k` 关联的 TSD 值设置为 `x`。包含 COW（写时复制）优化：若新旧值相同则跳过写入，避免不必要的页表操作。

#### 前置条件

- `k` 是有效的 `pthread_key_t` 值（0 <= k < PTHREAD_KEYS_MAX）
- 调用线程的 `self->tsd` 已初始化（非 NULL）

#### 后置条件

- Case 1 新旧值相同（`self->tsd[k] == x`）：
  - 不执行任何写操作（避免 COW 页面复制）
  - 不设置 `tsd_used` 标志
  - 返回 `0`
- Case 2 新旧值不同（`self->tsd[k] != x`）：
  - `self->tsd[k] = (void *)x`
  - `self->tsd_used = 1`（标记需要 TSD 析构）
  - 返回 `0`

#### 系统算法

```
pthread_setspecific(k, x):
  1. self = __pthread_self()
  2. 若 self->tsd[k] != x（避免不必要 COW）：
     a. self->tsd[k] = (void *)x
     b. self->tsd_used = 1
  3. 返回 0
```

#### 不变量

- 始终返回 `0`（POSIX 规定此函数不返回错误）
- `tsd_used` 标志保证线程退出时析构函数会被调用

#### 依赖

- `__pthread_self()` — 获取调用线程的 `pthread` 结构体指针
