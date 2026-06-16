# pthread_rwlockattr_init.c 规约

> musl libc 读写锁属性初始化函数。将 `pthread_rwlockattr_t` 对象初始化为默认值。

---

## 依赖图

```
pthread_rwlockattr_init (Public API)
  (无内部函数依赖，仅做零初始化)
```

---

## 函数规约

### 1. pthread_rwlockattr_init

```c
int pthread_rwlockattr_init(pthread_rwlockattr_t *a);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出（POSIX）

#### Intent

将读写锁属性对象 `a` 初始化为默认值。musl 实现中默认值为全零：`__attr[0] = 0` 表示 `PTHREAD_PROCESS_PRIVATE`。

#### 前置条件

- `a != NULL`，指向未初始化或可重新初始化的 `pthread_rwlockattr_t` 内存

#### 后置条件

- Case 1 成功：
  - `a->__attr[0] = 0`（默认进程私有）
  - `a->__attr[1] = 0`（保留字段）
  - 返回 0
- 无错误分支

#### 不变量

无。

#### 系统算法

```
pthread_rwlockattr_init(a):
  1. *a = (pthread_rwlockattr_t){0}  复合字面量零初始化
  2. return 0
```

#### 依赖

无内部依赖。依赖 C11 复合字面量语法。
