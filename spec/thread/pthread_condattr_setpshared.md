# pthread_condattr_setpshared.c 规约

> musl libc 设置条件变量属性中的进程共享标志。

---

## 依赖图

```
pthread_condattr_setpshared
  (无内部依赖)
```

---

## 函数规约

### 1. pthread_condattr_setpshared

```c
int pthread_condattr_setpshared(pthread_condattr_t *a, int pshared);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

设置条件变量是否可以跨进程共享。

#### 前置条件

- `a != NULL`，指向有效的 `pthread_condattr_t` 对象
- `pshared` 必须是 `PTHREAD_PROCESS_PRIVATE`（0）或 `PTHREAD_PROCESS_SHARED`（1）

#### 后置条件

- Case 1 `pshared == 0`（PTHREAD_PROCESS_PRIVATE）：
  - `a->__attr` 最高位清零（非进程共享）
  - 返回 `0`
- Case 2 `pshared == 1`（PTHREAD_PROCESS_SHARED）：
  - `a->__attr` 最高位置 1（进程共享）
  - `a->__attr` 低 31 位保持不变
  - 返回 `0`
- Case 3 `pshared > 1`（无效值）：
  - 返回 `EINVAL`
  - `a->__attr` 不变

#### 系统算法

```
pthread_condattr_setpshared(a, pshared):
  1. if pshared > 1U:
        return EINVAL
  2. a->__attr &= 0x7fffffff           // 清零最高位
  3. a->__attr |= (unsigned)pshared << 31  // 设置最高位 = pshared
  4. return 0
```

#### 不变量

- `a->__attr` 低 31 位的时钟 ID 在设置进程共享标志时不可丢失

#### 依赖

- `pthread_condattr_t` — 定义于 `<pthread.h>`，实质为 `struct { unsigned __attr; }`
- `EINVAL` — POSIX 错误码，来自 `<errno.h>`
