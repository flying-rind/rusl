# sem_trywait.c 规约

> musl libc 信号量非阻塞 P 操作（尝试递减）函数。

---

## 依赖图

```
sem_trywait
  └─> a_cas()     (see atomic.h — 原子比较并交换)
```

---

## 函数规约

### 1. `sem_trywait`

```c
int sem_trywait(sem_t *sem);
```

[Visibility]: User — 通过 `<semaphore.h>` 对外导出

#### Intent

非阻塞递减（锁定）信号量。若信号量值为 0，立即返回错误而不等待。

#### 前置条件

- `sem != NULL`，指向有效 `sem_t`

#### 后置条件

- Case 1 成功（信号量 > 0）：
  - `sem->__val[0]` 原子递减 1
  - 返回 `0`
- Case 2 失败（信号量 == 0）：
  - `errno = EAGAIN`
  - 返回 `-1`

#### 系统算法

```
sem_trywait(sem):
  1. 循环读取 val = sem->__val[0]
  2. 若 val & SEM_VALUE_MAX != 0（计数值 > 0）：
     - 尝试 CAS(sem->__val, val, val-1)
     - 若 CAS 成功 → 返回 0
     - 若 CAS 失败 → 重新读取 val（被其他线程并发修改）
  3. 若 val & SEM_VALUE_MAX == 0（计数值为 0）：
     - errno = EAGAIN
     - 返回 -1
```

#### 不变量

- 非阻塞操作，从不挂起调用线程
- CAS 循环确保并发安全（与 `sem_post` 等操作互不干扰）
- 仅当 `(val & SEM_VALUE_MAX) > 0` 时才尝试递减

#### 依赖

- `a_cas()` — 原子比较并交换（无锁并发控制）
