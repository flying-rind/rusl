# pthread_mutexattr_setprotocol.c 规约

> musl libc pthread 互斥锁属性：设置优先级协议（优先级继承 PI / 优先级保护）。优先级继承可缓解优先级反转问题。

---

## 依赖图

```
pthread_mutexattr_setprotocol
  ├── __syscall(SYS_futex, &lk, FUTEX_LOCK_PI, ...)  — 检测 PI futex 支持
  ├── a_store(&check_pi_result, r)                   — 原子写入缓存检测结果
  └── (静态变量 check_pi_result)                     — 惰性缓存内核能力检测
```

---

## 类型与常量

| 宏 | 值 | 含义 |
|----|-----|------|
| `PTHREAD_PRIO_NONE` | 0 | 无优先级协议 |
| `PTHREAD_PRIO_INHERIT` | 1 | 优先级继承（PI） |
| `PTHREAD_PRIO_PROTECT` | 2 | 优先级保护（musl 不支持） |

PI 标志存储在 `__attr` 的位 3（`__attr & 8`）。

---

## 内部符号

### 静态变量 check_pi_result

```c
static volatile int check_pi_result = -1;
```

[Visibility]: Internal (不导出) — 文件内静态变量

缓存内核 PI futex 特性的检测结果：`-1` 表示尚未检测，`0` 表示内核支持，正值为 `errno` 值表示不支持。

---

## 函数规约

### 1. pthread_mutexattr_setprotocol

```c
int pthread_mutexattr_setprotocol(pthread_mutexattr_t *a, int protocol);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

设置互斥锁属性对象的优先级协议。支持 `PTHREAD_PRIO_NONE`（无协议）和 `PTHREAD_PRIO_INHERIT`（优先级继承）两种模式。优先级继承可使高优先级线程在等待低优先级线程持有的互斥锁时，临时提升低优先级线程的优先级，从而缓解优先级反转。

#### 前置条件

- `a != NULL`，指向一个已初始化的 `pthread_mutexattr_t`

#### 后置条件

- Case 1 `protocol` 为 `PTHREAD_PRIO_NONE`(0)：
  - `a->__attr` 的位 3 被清除
  - 返回值为 `0`
- Case 2 `protocol` 为 `PTHREAD_PRIO_INHERIT`(1)，且内核支持 PI futex：
  - `a->__attr` 的位 3 被设置
  - 返回值为 `0`
- Case 3 `protocol` 为 `PTHREAD_PRIO_INHERIT`(1)，但内核不支持 PI futex：
  - `a->__attr` 不变
  - 返回值为内核返回的 `errno`（如 `ENOSYS`）
- Case 4 `protocol` 为 `PTHREAD_PRIO_PROTECT`(2)：
  - `a->__attr` 不变
  - 返回值为 `ENOTSUP`
- Case 5 `protocol` 为其他非法值：
  - `a->__attr` 不变
  - 返回值为 `EINVAL`

#### 系统算法

```
pthread_mutexattr_setprotocol(a, protocol):
  1. switch protocol:
  2.   case PTHREAD_PRIO_NONE(0):
       a->__attr &= ~8
       return 0
  3.   case PTHREAD_PRIO_INHERIT(1):
       r = check_pi_result
       if r < 0:  // 惰性检测
          volatile int lk = 0
          r = -__syscall(SYS_futex, &lk, FUTEX_LOCK_PI, 0, 0)
          a_store(&check_pi_result, r)
       if r != 0: return r  // 内核不支持
       a->__attr |= 8
       return 0
  4.   case PTHREAD_PRIO_PROTECT(2):
       return ENOTSUP
  5.   default:
       return EINVAL
```

#### 不变量

- `check_pi_result` 在首次写入后保持不变（惰性缓存）
- `lk` 被初始化为 0 且不会被实际修改（PI lock 仅用于探测，立即 unlock）
- 非 PI 属性位（type、robust、process-shared 位）不受此操作影响

#### 依赖

| 接口 | 来源 | 说明 |
|------|------|------|
| `__syscall(SYS_futex, ...)` | 内核系统调用 | 探测 PI futex 支持 |
| `FUTEX_LOCK_PI` (6) | `futex.h` (内部) | PI futex 加锁操作 |
| `a_store(p, v)` | `atomic.h` (内部) | 原子写入 |
| `check_pi_result` | 文件内静态变量 | 惰性缓存内核支持结果 |
