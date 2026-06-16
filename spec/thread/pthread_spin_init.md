# pthread_spin_init.c 规约

> musl libc 的自旋锁初始化函数实现。将自旋锁值置为 0（未锁定状态）。

---

## 依赖图

```
pthread_spin_init
  (无内部依赖)
```

---

## 函数规约

### 1. pthread_spin_init

```c
int pthread_spin_init(pthread_spinlock_t *s, int shared);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

初始化一个自旋锁对象。`pthread_spinlock_t` 在 musl 中定义为 `int`，初始化值 0 表示"解锁/可用"状态。

#### 前置条件

- `s != NULL`，指向待初始化的自旋锁对象
- `shared` 为 `PTHREAD_PROCESS_PRIVATE` (0) 或 `PTHREAD_PROCESS_SHARED` (1)

#### 后置条件

- `*s = 0`（锁处于解锁状态）
- 返回值为 `*s` 即 0（成功）—— C 赋值表达式的值
- musl 中 `shared` 参数被忽略（自旋锁仅在进程内有效，不支持跨进程共享）

#### 系统算法

```
pthread_spin_init(s, shared):
  1. *s = 0      // 初始化为解锁状态
  2. return *s   // 返回 0 (成功)
```

#### 不变量

无。

#### 依赖

无内部依赖。仅依赖 `<pthread.h>` 中定义的 `pthread_spinlock_t` 类型。
