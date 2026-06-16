# pthread_getconcurrency.c 规约

> musl libc 获取建议并发级别（已废弃的 POSIX 函数）。在 Linux 上固定返回 0。

---

## 依赖图

```
pthread_getconcurrency  (无内部依赖)
```

---

## 函数规约

### 1. pthread_getconcurrency

```c
int pthread_getconcurrency(void);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

获取应用程序建议的线程并发级别。该函数在 POSIX.1-2001 和 POSIX.1-2008 中已被标记为废弃，musl 始终返回 0，表示由操作系统自行管理并发。

#### 前置条件

- 无

#### 后置条件

- 始终返回 0
- 无副作用：不修改任何全局或静态状态

#### 系统算法

```
pthread_getconcurrency():
  1. return 0
```

#### 不变量

无。该函数不访问任何全局状态。

#### 依赖

- 无外部或内部依赖
