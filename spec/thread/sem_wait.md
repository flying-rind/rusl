# sem_wait.c 规约

> musl libc 信号量阻塞 P 操作（无限等待）函数。

---

## 依赖图

```
sem_wait
  └─> sem_timedwait(sem, NULL)    (see sem_timedwait.c spec)
```

---

## 函数规约

### 1. `sem_wait`

```c
int sem_wait(sem_t *sem);
```

[Visibility]: User — 通过 `<semaphore.h>` 对外导出

#### Intent

阻塞递减（锁定）信号量。若信号量值为 0，则挂起当前线程直到信号量变为可用或被取消。等价于 `sem_timedwait(sem, NULL)`（无限等待）。

#### 前置条件

- `sem != NULL`，指向有效 `sem_t`

#### 后置条件

- Case 1 成功：
  - 信号量计数值原子递减 1
  - 返回 `0`
- Case 2 线程被取消（取消点）：
  - 线程不返回，等待者计数正确递减

#### 系统算法

```
sem_wait(sem):
  直接返回 sem_timedwait(sem, 0)（即 at == NULL，无限等待）
```

#### 不变量

- 完全委托给 `sem_timedwait` 实现，`at == NULL` 时无超时
- 属于 POSIX 取消点（`sem_timedwait` 内部调用 `pthread_testcancel`）

#### 依赖

- `sem_timedwait()` — 带超时的信号量递减函数（`at == NULL` 时无限等待）
