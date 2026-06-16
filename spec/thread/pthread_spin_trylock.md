# pthread_spin_trylock.c 规约

> musl libc 的自旋锁尝试锁定函数实现。使用单次原子 CAS 操作，不阻塞不等待。如果锁已被占用则立即返回 EBUSY。

---

## 依赖图

```
pthread_spin_trylock
  └─> a_cas(s, 0, EBUSY)  — see internal/atomic.h (原子 CAS)
```

---

## 函数规约

### 1. pthread_spin_trylock

```c
int pthread_spin_trylock(pthread_spinlock_t *s);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

尝试以非阻塞方式获取自旋锁。若锁空闲则获取并返回 0，若锁已被占用则立即返回错误而不等待。

#### 前置条件

- `s != NULL`，指向通过 `pthread_spin_init` 初始化的自旋锁对象

#### 后置条件

- Case 1 锁空闲（`*s == 0`）：CAS 原子地将 `*s` 从 0 设为 `EBUSY`，返回 0（旧值为 0 = 成功）
- Case 2 锁被占用（`*s != 0`）：CAS 失败，`*s` 不变，返回旧值即 `EBUSY`（非零值）
- 未获取到锁时调用线程不会阻塞

#### 系统算法

```
pthread_spin_trylock(s):
  1. return a_cas(s, 0, EBUSY)
     // 若 *s == 0: 原子地将 *s 设为 EBUSY, 返回 0 (成功)
     // 若 *s != 0: 不变, 返回 *s 的旧值 (即 EBUSY)
```

#### 不变量

- 自旋锁状态仅在空闲（0）和占用（`EBUSY`）之间原子转换

#### 依赖

- `a_cas(p, t, s)` — 原子比较并交换：若 `*p == t` 则 `*p = s`，返回旧值（见 `internal/atomic.h`）
- `EBUSY` — errno 值，用作锁状态标记（来自 `<errno.h>`）
