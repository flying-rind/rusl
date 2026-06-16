# pthread_mutexattr_init.c 规约

> musl libc pthread 互斥锁属性对象初始化。将 `pthread_mutexattr_t` 设置为默认值，是所有互斥锁属性操作的起点。

---

## 依赖图

```
pthread_mutexattr_init
  └── (无依赖 — 仅做零初始化)
```

---

## 类型与结构体

### pthread_mutexattr_t

```c
// 定义于 <bits/alltypes.h.in>，通过 <pthread.h> 引入
struct { unsigned __attr; } pthread_mutexattr_t;
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

`__attr` 字段编码互斥锁属性位掩码，各位含义如下：

| 位 | 值 | 含义 |
|----|-----|------|
| 0–1 ( `& 3` ) | — | mutex type: 0=NORMAL/DEFAULT, 1=RECURSIVE, 2=ERRORCHECK |
| 2 ( `& 4` ) | `PTHREAD_MUTEX_ROBUST` | 健壮性标志 |
| 3 ( `& 8` ) | `PTHREAD_PRIO_INHERIT` | 优先级继承标志 |
| 7 ( `& 128` ) | `PTHREAD_PROCESS_SHARED` | 进程共享标志 |

默认值 `{0}` 对应：`NORMAL` 类型、非 robust、非 PI、进程私有。

---

## 函数规约

### 1. pthread_mutexattr_init

```c
int pthread_mutexattr_init(pthread_mutexattr_t *a);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

#### Intent

将互斥锁属性对象 `a` 初始化为默认值。默认互斥锁为非递归、非健壮、非优先级继承的进程私有 NORMAL 互斥锁。

#### 前置条件

- `a != NULL`，指向调用者分配的 `pthread_mutexattr_t` 内存

#### 后置条件

- Case 1（总是成功）：
  - `a->__attr == 0`
  - 返回值为 `0`

#### 系统算法

```
pthread_mutexattr_init(a):
  1. *a = (pthread_mutexattr_t){0}   // 复合字面量零初始化全部字段
  2. return 0
```

#### 不变量

无。

#### 依赖

无外部依赖。仅使用 C 复合字面量语法进行零初始化。
