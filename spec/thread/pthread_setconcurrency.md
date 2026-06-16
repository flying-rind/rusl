# pthread_setconcurrency.c 规约

> musl libc 设置建议并发级别（已废弃的 POSIX 函数）。参数校验后，非零值均返回失败。

---

## 依赖图

```
pthread_setconcurrency  (无内部依赖)
```

---

## 函数规约

### 1. pthread_setconcurrency

```c
int pthread_setconcurrency(int val);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

设置应用程序建议的线程并发级别。该函数在 POSIX.1-2001 和 POSIX.1-2008 中已被标记为废弃。musl 的实现仅做参数合法性校验：val < 0 返回 EINVAL，val > 0 返回 EAGAIN，val == 0 返回 0。

#### 前置条件

- 无

#### 后置条件

- Case 1 (val < 0): 返回 `EINVAL`，表示参数非法
- Case 2 (val > 0): 返回 `EAGAIN`，表示指定的并发级别当前无法满足
- Case 3 (val == 0): 返回 0，成功（操作系统自行管理并发）
- 无副作用：不修改任何全局或静态状态

#### 系统算法

```
pthread_setconcurrency(val):
  if val < 0:  return EINVAL
  if val > 0:  return EAGAIN
  return 0
```

#### 不变量

无。该函数不访问任何全局状态。

#### 依赖

- `<errno.h>` — `EINVAL`, `EAGAIN` 宏定义
