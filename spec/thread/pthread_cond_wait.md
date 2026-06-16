# pthread_cond_wait.c 规约

> musl libc 条件变量无限等待函数。是 `pthread_cond_timedwait` 的简单包装。

---

## 依赖图

```
pthread_cond_wait
  └─> pthread_cond_timedwait(c, m, 0)  (User, 定义于 pthread_cond_timedwait.c)
```

---

## 函数规约

### 1. pthread_cond_wait

```c
int pthread_cond_wait(pthread_cond_t *restrict c, pthread_mutex_t *restrict m);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

在条件变量上无限期阻塞当前线程，原子释放 mutex 并进入等待，直至被 signal/broadcast 唤醒。等价于 `pthread_cond_timedwait(c, m, NULL)`。

#### 前置条件

- `c != NULL`，指向有效 `pthread_cond_t`
- `m != NULL`，指向已由当前线程锁定的 `pthread_mutex_t`

#### 后置条件

- 返回时 mutex 已由当前线程重新锁定
- Case 1 正常被唤醒：返回 `0`
- Case 2 被取消：返回 `ECANCELED`（受 __pthread_cond_timedwait 取消规则约束）
- Case 3 被信号中断：返回 `EINTR`
- Case 4 mutex 校验失败：返回 `EPERM`

#### 系统算法

```
pthread_cond_wait(c, m):
  1. return pthread_cond_timedwait(c, m, 0)  // ts=NULL 表示无限等待
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `pthread_cond_timedwait()` — 用户可见的条件变量超时等待函数（定义于 `pthread_cond_timedwait.c`，见其 spec）
