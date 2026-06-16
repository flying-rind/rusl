# pthread_mutex_lock.c 规约

> musl libc pthread 互斥锁加锁（阻塞）。NORMAL 类型尝试快速 CAS 路径，失败后委托给 `__pthread_mutex_timedlock` 完成阻塞等待。

---

## 依赖图

```
__pthread_mutex_lock
  ├── a_cas(&m->_m_lock, 0, EBUSY)          — NORMAL 类型快速路径
  └── __pthread_mutex_timedlock(m, 0)       — 通用阻塞加锁路径 (NULL 超时 = 无限等待)

pthread_mutex_lock  (weak_alias → __pthread_mutex_lock)
```

---

## 函数规约

### 1. __pthread_mutex_lock / pthread_mutex_lock

```c
int __pthread_mutex_lock(pthread_mutex_t *m);
int pthread_mutex_lock(pthread_mutex_t *m);  // weak_alias
```

[Visibility]:
- `__pthread_mutex_lock`: Internal (不导出) — musl 内部实现符号
- `pthread_mutex_lock`: User — 通过 `<pthread.h>` 对外导出（weak_alias）

#### Intent

以阻塞方式获取互斥锁。若互斥锁已被其他线程持有，调用线程将阻塞直到互斥锁可用。

#### 前置条件

- `m != NULL`，指向一个已初始化的 `pthread_mutex_t`

#### 后置条件

- Case 1 成功获取：
  - 互斥锁被调用线程锁定
  - 若为 RECURSIVE 类型且已由调用线程持有：`_m_count` 递增
  - 返回值为 `0`
- Case 2 死锁检测（ERRORCHECK 类型重复加锁）：
  - 返回值为 `EDEADLK`
- Case 3 互斥锁处于 EOWNERDEAD 状态（robust）：
  - 互斥锁被获取，返回值为 `EOWNERDEAD`
- Case 4 被信号中断：
  - 可能被重启，取决于具体实现路径

#### 系统算法

```
__pthread_mutex_lock(m):
  1. // NORMAL 类型快速路径
     if (m->_m_type & 15) == PTHREAD_MUTEX_NORMAL:
       if a_cas(&m->_m_lock, 0, EBUSY) == 0:
         return 0       // 无竞争，立即获取
  2. // 通用路径：委托给 timedlock (超时为 NULL = 无限等待)
     return __pthread_mutex_timedlock(m, 0)
```

#### 不变量

- 加锁成功后 `_m_lock` 记录了当前线程的 tid

#### 依赖

| 接口 | 来源 | 说明 |
|------|------|------|
| `a_cas(p, t, s)` | `atomic.h` (内部) | 原子比较交换 |
| `__pthread_mutex_timedlock(m, 0)` | 本模块 (pthread_mutex_timedlock.c) | 带超时的通用加锁，NULL = 无限等待 |
| `EBUSY` (16) | `<errno.h>` | NORMAL 锁的"已锁定"标记 |
| `PTHREAD_MUTEX_NORMAL` (0) | `<pthread.h>` | NORMAL 锁类型常量 |
