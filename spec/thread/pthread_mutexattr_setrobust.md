# pthread_mutexattr_setrobust.c 规约

> musl libc pthread 互斥锁属性：设置健壮性（robust）标志。健壮互斥锁在持有者线程终止时能被后续加锁者检测到并恢复。

---

## 依赖图

```
pthread_mutexattr_setrobust
  ├── __syscall(SYS_get_robust_list, ...)  — 检测内核 robust list 支持
  ├── a_store(&check_robust_result, r)    — 原子写入缓存检测结果
  └── (静态变量 check_robust_result)       — 惰性缓存内核能力检测
```

---

## 类型与常量

| 宏 | 值 | 含义 |
|----|-----|------|
| `PTHREAD_MUTEX_STALLED` | 0 | 非健壮（默认） |
| `PTHREAD_MUTEX_ROBUST` | 1 | 健壮互斥锁 |

健壮标志存储在 `__attr` 的位 2（`__attr & 4`）。

---

## 内部符号

### 静态变量 check_robust_result

```c
static volatile int check_robust_result = -1;
```

[Visibility]: Internal (不导出) — 文件内静态变量

缓存内核 robust list 特性的检测结果：`-1` 表示尚未检测，`0` 表示内核支持，正值为 `errno` 值表示不支持。

---

## 函数规约

### 1. pthread_mutexattr_setrobust

```c
int pthread_mutexattr_setrobust(pthread_mutexattr_t *a, int robust);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

设置互斥锁属性对象的健壮性标志。当 `PTHREAD_MUTEX_ROBUST` 被设置时，若互斥锁的持有者线程异常终止，后续加锁操作可检测到此状态并恢复。

#### 前置条件

- `a != NULL`，指向一个已初始化的 `pthread_mutexattr_t`

#### 后置条件

- Case 1 `robust` 为 `PTHREAD_MUTEX_STALLED`(0)：
  - `a->__attr` 的位 2 被清除（`__attr &= ~4`）
  - 返回值为 `0`
- Case 2 `robust` 为 `PTHREAD_MUTEX_ROBUST`(1)，且内核支持 robust list：
  - `a->__attr` 的位 2 被设置（`__attr |= 4`）
  - 返回值为 `0`
- Case 3 `robust` 为 `PTHREAD_MUTEX_ROBUST`(1)，但内核不支持 robust list：
  - `a->__attr` 不变
  - 返回值为内核返回的 `errno`（如 `ENOSYS`）
- Case 4 `robust > 1`（非法值）：
  - `a->__attr` 不变
  - 返回值为 `EINVAL`

#### 系统算法

```
pthread_mutexattr_setrobust(a, robust):
  1. if robust > 1U: return EINVAL
  2. if robust == 0:
       a->__attr &= ~4
       return 0
  3. // robust == 1: 惰性检测内核支持
     r = check_robust_result
     if r < 0:  // 尚未检测
       r = -__syscall(SYS_get_robust_list, 0, &p, &l)
       a_store(&check_robust_result, r)
     if r != 0: return r       // 内核不支持
     a->__attr |= 4
     return 0
```

#### 不变量

- `check_robust_result` 在首次写入后保持不变（惰性缓存）
- 非 robust 属性位（type、PI、process-shared 位）不受此操作影响

#### 依赖

| 接口 | 来源 | 说明 |
|------|------|------|
| `__syscall(SYS_get_robust_list, ...)` | 内核系统调用 | 探测内核是否支持 robust list |
| `a_store(p, v)` | `atomic.h` (内部) | 原子写入 |
| `check_robust_result` | 文件内静态变量 | 惰性缓存内核支持结果 |
