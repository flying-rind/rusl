# pthread_barrier_init.c 规约

> musl libc 屏障对象初始化函数。

---

## 依赖图

```
pthread_barrier_init
  (无内部依赖)
```

---

## 函数规约

### 1. pthread_barrier_init

```c
int pthread_barrier_init(pthread_barrier_t *restrict b, const pthread_barrierattr_t *restrict a, unsigned count);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

初始化屏障对象，设置参与屏障的线程数 `count`。若提供属性对象，根据属性设置进程共享标志。屏障要求 `count` 个线程全部到达后才能继续。

#### 前置条件

- `b != NULL`，指向未初始化的 `pthread_barrier_t` 对象
- `a` 可以为 `NULL`（默认属性）或指向有效的 `pthread_barrierattr_t`
- `count > 0`

#### 后置条件

- Case 1 `count-1 <= INT_MAX-1`（即 `count <= INT_MAX`）：
  - `b->_b_limit = count - 1`，或上可选属性 `a->__attr`
  - 返回 `0`
- Case 2 `count-1 > INT_MAX-1`（即 `count > INT_MAX`）：
  - 返回 `EINVAL`
  - `*b` 不变

#### 字段编码说明

- `_b_limit` 存储 `count-1 | attr.__attr`
- 进程共享屏障：`attr.__attr == INT_MIN`，因此 `_b_limit < 0`（最高位为 1）→ 在 `pshared_barrier_wait` 中通过 `_b_limit < 0` 检测
- 非进程共享屏障：`attr.__attr == 0`，因此 `_b_limit == count-1 >= 0`

#### 系统算法

```
pthread_barrier_init(b, a, count):
  1. if count-1 > INT_MAX-1:
        return EINVAL
  2. *b = (pthread_barrier_t){ ._b_limit = count-1 | (a ? a->__attr : 0) }
  3. return 0
```

#### 不变量

- 初始化后的 `_b_limit` 字段编码了 count-1 和进程共享标志
- 其余字段（`_b_lock`, `_b_waiters`, `_b_count`, `_b_waiters2`, `_b_inst`）由复合字面量零初始化

#### 依赖

- `pthread_barrier_t` — 定义于 `<pthread.h>`，内含 `_b_limit` 等字段
- `pthread_barrierattr_t` — 定义于 `<pthread.h>`，实质为 `struct { unsigned __attr; }`
- `INT_MAX` — 来自 `<limits.h>`
- `EINVAL` — POSIX 错误码，来自 `<errno.h>`
