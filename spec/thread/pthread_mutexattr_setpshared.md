# pthread_mutexattr_setpshared.c 规约

> musl libc pthread 互斥锁属性：设置进程共享标志，决定互斥锁是进程私有还是跨进程共享。

---

## 依赖图

```
pthread_mutexattr_setpshared
  └── (无系统调用或函数调用依赖)
```

---

## 类型与常量

| 宏 | 值 | 含义 |
|----|-----|------|
| `PTHREAD_PROCESS_PRIVATE` | 0 | 仅同进程内各线程间共享 |
| `PTHREAD_PROCESS_SHARED` | 1 | 跨进程共享（需放置在共享内存中） |

共享标志存储在 `__attr` 的位 7（`__attr & 128`）。`pshared` 值为 0 或 1，左移 7 位存入。

---

## 函数规约

### 1. pthread_mutexattr_setpshared

```c
int pthread_mutexattr_setpshared(pthread_mutexattr_t *a, int pshared);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

设置互斥锁属性对象的进程共享属性。`PTHREAD_PROCESS_SHARED` 表示互斥锁可在不同进程间同步（须位于共享内存段中）；`PTHREAD_PROCESS_PRIVATE` 为默认值。

#### 前置条件

- `a != NULL`，指向一个已初始化的 `pthread_mutexattr_t`

#### 后置条件

- Case 1 `pshared` 为 `PTHREAD_PROCESS_PRIVATE`(0) 或 `PTHREAD_PROCESS_SHARED`(1)：
  - `a->__attr` 的位 7 设置为 `pshared << 7`，其他位保持不变
  - 返回值为 `0`
- Case 2 `pshared > 1`（非法值）：
  - `a->__attr` 不变
  - 返回值为 `EINVAL`

#### 系统算法

```
pthread_mutexattr_setpshared(a, pshared):
  1. if pshared > 1U: return EINVAL
  2. a->__attr &= ~128U          // 清除位 7
  3. a->__attr |= pshared << 7   // 设置位 7
  4. return 0
```

#### 不变量

- 非 process-shared 属性位（type、robust、PI 位）不受此操作影响

#### 依赖

无外部依赖。
