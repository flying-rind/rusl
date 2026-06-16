# pthread_mutex_getprioceiling.c 规约

> musl libc pthread 互斥锁获取优先级天花板。musl 不支持 `PTHREAD_PRIO_PROTECT` 协议，此函数始终返回 EINVAL。

---

## 依赖图

```
pthread_mutex_getprioceiling
  └── (无依赖 — 直接返回 EINVAL)
```

---

## 函数规约

### 1. pthread_mutex_getprioceiling

```c
int pthread_mutex_getprioceiling(const pthread_mutex_t *restrict m, int *restrict ceiling);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

POSIX 定义此函数用于获取互斥锁的优先级天花板值。该功能依赖 `PTHREAD_PRIO_PROTECT` 协议（优先级保护），musl 不支持此协议，因此函数始终返回 `EINVAL` 以表明此功能不可用。

#### 前置条件

- `m != NULL`
- `ceiling != NULL`

#### 后置条件

- Case 1（总是）：
  - 返回值为 `EINVAL`
  - `*ceiling` 不变

#### 系统算法

```
pthread_mutex_getprioceiling(m, ceiling):
  1. return EINVAL  // musl 不支持 PTHREAD_PRIO_PROTECT
```

#### 不变量

无。

#### 依赖

| 接口 | 来源 | 说明 |
|------|------|------|
| `EINVAL` (22) | `<errno.h>` | 无效参数错误码 |
