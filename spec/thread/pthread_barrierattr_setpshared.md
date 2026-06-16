# pthread_barrierattr_setpshared.c 规约

> musl libc 设置屏障属性中的进程共享标志。

---

## 依赖图

```
pthread_barrierattr_setpshared
  (无内部依赖)
```

---

## 函数规约

### 1. pthread_barrierattr_setpshared

```c
int pthread_barrierattr_setpshared(pthread_barrierattr_t *a, int pshared);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

设置屏障是否可以跨进程共享。`pshared=0` 表示 `PTHREAD_PROCESS_PRIVATE`，`pshared=1` 表示 `PTHREAD_PROCESS_SHARED`。

#### 前置条件

- `a != NULL`，指向有效的 `pthread_barrierattr_t` 对象
- `pshared` 必须是 `0` 或 `1`

#### 后置条件

- Case 1 `pshared == 0`：`a->__attr = 0`，返回 `0`
- Case 2 `pshared == 1`：`a->__attr = INT_MIN`（即 `0x80000000`），返回 `0`
- Case 3 `pshared > 1`：返回 `EINVAL`，`a->__attr` 不变

注意：此处 `__attr` 设置逻辑不同于 `pthread_condattr_setpshared`。屏障属性直接将 `__attr` 整体设为 `0` 或 `INT_MIN`（而非仅设置最高位）。这是因为屏障属性仅编码进程共享标志，没有其他位需要保留。

#### 系统算法

```
pthread_barrierattr_setpshared(a, pshared):
  1. if pshared > 1U:
        return EINVAL
  2. a->__attr = pshared ? INT_MIN : 0
  3. return 0
```

#### 不变量

无。

#### 依赖

- `pthread_barrierattr_t` — 定义于 `<pthread.h>`，实质为 `struct { unsigned __attr; }`
- `INT_MIN` — 来自 `<limits.h>`
- `EINVAL` — POSIX 错误码，来自 `<errno.h>`
