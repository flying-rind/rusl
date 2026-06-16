# pthread_rwlock_init.c 规约

> musl libc 读写锁初始化函数。使用可选的属性对象初始化 `pthread_rwlock_t`。

---

## 依赖图

```
pthread_rwlock_init (Public API)
  (无内部函数依赖，仅做字段赋值)
```

---

## 数据结构

### pthread_rwlock_t 内部字段

```c
typedef struct {
    union {
        int __i[...];
        volatile int __vi[...];
        void *__p[...];
    } __u;
} pthread_rwlock_t;

#define _rw_lock    __u.__vi[0]   // 锁状态字
#define _rw_waiters __u.__vi[1]   // 等待者计数
#define _rw_shared  __u.__i[2]    // 进程共享标志
```

锁状态字 `_rw_lock` 语义：
- `0`: 未加锁
- `1` ~ `0x7ffffffe`: 持有读锁的线程数
- `0x7fffffff` (INT32_MAX): 写锁持有中
- bit 31 (`0x80000000`): 等待者标志（用于 futex wait 值编码）

---

## 函数规约

### 1. pthread_rwlock_init

```c
int pthread_rwlock_init(pthread_rwlock_t *restrict rw, const pthread_rwlockattr_t *restrict a);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出（POSIX）

#### Intent

初始化读写锁对象 `rw`。若 `a` 为 NULL，使用默认属性（进程私有）；否则从 `a` 读取进程共享设置。

#### 前置条件

- `rw != NULL`，指向未初始化或可重新初始化的内存
- `a` 为 NULL 或指向已初始化的有效 `pthread_rwlockattr_t` 对象
- `rw` 和 `a` 不重叠（restrict 约束）

#### 后置条件

- Case 1 成功：
  - `rw` 被零初始化：`_rw_lock = 0`, `_rw_waiters = 0`, `_rw_shared = 0`
  - 若 `a != NULL`：`rw->_rw_shared = a->__attr[0] * 128`
  - 返回 0
- 无错误分支

#### 系统算法

```
pthread_rwlock_init(rw, a):
  1. *rw = (pthread_rwlock_t){0}           零初始化整个结构体
  2. if (a != NULL)
       rw->_rw_shared = a->__attr[0] * 128  0 或 128 (FUTEX_PRIVATE = 128)
  3. return 0
```

`_rw_shared` 字段的编码约定：
- `0`: 进程共享模式（futex 操作使用 `FUTEX_WAIT` / `FUTEX_WAKE`）
- `128`: 进程私有模式（futex 操作使用 `FUTEX_WAIT | FUTEX_PRIVATE` / `FUTEX_WAKE | FUTEX_PRIVATE`）
- 在 `__wake`、`__timedwait` 调用中通过 `priv = rw->_rw_shared ^ 128` 提取出 `0` 或 `128` 作为 futex private 标志

#### 依赖

- `pthread_rwlock_t` / `pthread_rwlockattr_t` — 来自 `<alltypes.h.in>` 的类型定义
- C11 复合字面量语法
