# pthread_rwlockattr_destroy.c 规约

> musl libc 读写锁属性销毁函数。销毁一个 `pthread_rwlockattr_t` 对象并释放其占用的实现定义资源。

---

## 依赖图

```
pthread_rwlockattr_destroy (Public API)
  (无内部依赖，直接返回 0)
```

---

## 数据结构

### pthread_rwlockattr_t

```c
typedef struct { unsigned __attr[2]; } pthread_rwlockattr_t;
```

读写锁属性对象，包含两个 unsigned int 字段：
- `__attr[0]`: 进程共享属性 (`PTHREAD_PROCESS_PRIVATE` = 0, `PTHREAD_PROCESS_SHARED` = 1)
- `__attr[1]`: 保留

---

## 函数规约

### 1. pthread_rwlockattr_destroy

```c
int pthread_rwlockattr_destroy(pthread_rwlockattr_t *a);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出（POSIX）

#### Intent

销毁读写锁属性对象 `a`。musl 实现中该结构体不持有动态分配的资源，因此该函数仅返回 0。

#### 前置条件

- `a` 指向一个已初始化的有效 `pthread_rwlockattr_t` 对象
- 在销毁后不应再次使用 `a`，除非重新初始化

#### 后置条件

- Case 1 成功：返回 0
- 无错误分支（musl 实现总是返回 0）

#### 不变量

无。

#### 依赖

无内部依赖。仅需 `<pthread.h>` 声明的类型定义。
