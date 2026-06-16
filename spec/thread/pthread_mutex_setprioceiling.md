# pthread_mutex_setprioceiling.c 规约

> musl libc pthread 互斥锁设置优先级天花板并加锁。musl 不支持 `PTHREAD_PRIO_PROTECT` 协议，此函数始终返回 EINVAL。

---

## 依赖图

```
pthread_mutex_setprioceiling
  └── (无依赖 — 直接返回 EINVAL)
```

---

## 函数规约

### 1. pthread_mutex_setprioceiling

```c
int pthread_mutex_setprioceiling(pthread_mutex_t *restrict m, int ceiling, int *restrict old);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

POSIX 定义此函数用于设置互斥锁的优先级天花板并原子地加锁。该功能依赖 `PTHREAD_PRIO_PROTECT` 协议（优先级保护），musl 不支持此协议，因此函数始终返回 `EINVAL` 以表明此功能不可用。

根据 POSIX 规范，支持此函数的实现应在加锁前设置天花板、或以原子操作同时加锁并设置天花板。musl 选择完全不支持。

#### 前置条件

- `m != NULL`
- `old` 可以为 NULL

#### 后置条件

- Case 1（总是）：
  - 返回值为 `EINVAL`
  - 互斥锁状态不变
  - `*old`（若非 NULL）不变

#### 系统算法

```
pthread_mutex_setprioceiling(m, ceiling, old):
  1. return EINVAL  // musl 不支持 PTHREAD_PRIO_PROTECT
```

#### 不变量

无。

#### 依赖

| 接口 | 来源 | 说明 |
|------|------|------|
| `EINVAL` (22) | `<errno.h>` | 无效参数错误码 |
