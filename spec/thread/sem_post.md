# sem_post.c 规约

> musl libc 信号量 V 操作（释放/递增）函数。

---

## 依赖图

```
sem_post
  ├─> a_cas()     (see atomic.h — 原子比较并交换)
  └─> __wake()    (see pthread_impl.h — futex wake)
```

---

## 函数规约

### 1. `sem_post`

```c
int sem_post(sem_t *sem);
```

[Visibility]: User — 通过 `<semaphore.h>` 对外导出

#### Intent

解锁/释放信号量，将计数值加 1。若有等待线程则唤醒。

#### 前置条件

- `sem != NULL`，指向有效 `sem_t`

#### 后置条件

- Case 1 成功（计数值 < `SEM_VALUE_MAX`）：
  - `sem->__val[0]` 原子递增 1
  - 若存在等待者（`waiters > 0` 或旧值为负），通过 futex wake 唤醒
  - 返回 `0`
- Case 2 溢出（计数值已达 `SEM_VALUE_MAX`）：
  - `errno = EOVERFLOW`
  - 返回 `-1`

#### 系统算法

```
sem_post(sem):
  1. priv = sem->__val[2]（futex 私有/共享标志）
  2. 循环（CAS 原子操作）：
     a. val = sem->__val[0]
     b. waiters = sem->__val[1]
     c. 若 (val & SEM_VALUE_MAX) == SEM_VALUE_MAX → errno = EOVERFLOW，返回 -1
     d. new = val + 1
     e. 若 waiters <= 1 → new &= ~0x80000000（清除等待标记位）
     f. 若 a_cas(sem->__val, val, new) == val → 跳出循环（CAS 成功）
  3. 若 val < 0 或 waiters > 0：
     - 调用 __wake(sem->__val, waiters, priv) 唤醒等待线程
     - 若 waiters > 1 → 唤醒 1 个（cnt=1）；否则唤醒所有（cnt=-1 → INT_MAX）
  4. 返回 0
```

#### 不变量

- 信号量计数值始终在 `0` 到 `SEM_VALUE_MAX`（0x7FFFFFFF）之间
- `sem->__val[0]` 低 31 位为计数值，bit 31 为内部等待标记
- 当 `waiters <= 1` 时清除 bit 31（新值为纯计数值），否则保留（指示仍有等待者）
- CAS 循环确保并发 `sem_post` / `sem_wait` 操作互不干扰

#### 依赖

- `a_cas()` — 原子比较并交换（并发安全的关键）
- `__wake()` — futex wake 系统调用（唤醒等待线程）
