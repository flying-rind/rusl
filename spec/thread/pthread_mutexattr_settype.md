# pthread_mutexattr_settype.c 规约

> musl libc pthread 互斥锁属性：设置互斥锁类型（NORMAL / RECURSIVE / ERRORCHECK）。

---

## 依赖图

```
pthread_mutexattr_settype
  └── (无系统调用或函数调用依赖)
```

---

## 类型与常量

| 宏 | 值 | 含义 |
|----|-----|------|
| `PTHREAD_MUTEX_NORMAL` | 0 | 普通锁（无死锁检测，无递归） |
| `PTHREAD_MUTEX_DEFAULT` | 0 | `PTHREAD_MUTEX_NORMAL` 的同义名 |
| `PTHREAD_MUTEX_RECURSIVE` | 1 | 允许同一线程重复加锁 |
| `PTHREAD_MUTEX_ERRORCHECK` | 2 | 检测死锁（重复加锁返回 EDEADLK） |

类型值存储在 `__attr` 的低 2 位（`__attr & 3`）。

---

## 函数规约

### 1. pthread_mutexattr_settype

```c
int pthread_mutexattr_settype(pthread_mutexattr_t *a, int type);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

设置互斥锁属性对象的互斥锁类型，决定后续创建的互斥锁在重复加锁时的行为（普通/递归/错误检测）。

#### 前置条件

- `a != NULL`，指向一个已初始化的 `pthread_mutexattr_t`

#### 后置条件

- Case 1 `type` 为 `PTHREAD_MUTEX_NORMAL`(0)、`PTHREAD_MUTEX_RECURSIVE`(1) 或 `PTHREAD_MUTEX_ERRORCHECK`(2)：
  - `a->__attr` 的低 2 位被替换为 `type`，其他位保持不变
  - 返回值为 `0`
- Case 2 `type` 为其他非法值：
  - `a->__attr` 不变
  - 返回值为 `EINVAL`

#### 系统算法

```
pthread_mutexattr_settype(a, type):
  1. if (unsigned)type > 2: return EINVAL
  2. a->__attr = (a->__attr & ~3) | type   // 清除低 2 位，设置新 type
  3. return 0
```

#### 不变量

- 非 type 属性位（robust 位、PI 位、process-shared 位）不受此操作影响

#### 依赖

无外部依赖。
