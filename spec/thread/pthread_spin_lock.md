# pthread_spin_lock.c 规约

> musl libc 的自旋锁锁定函数实现。使用原子比较并交换 (CAS) 操作，在忙等待循环中自旋直到获取锁。

---

## 依赖图

```
pthread_spin_lock
  ├─> a_cas(s, 0, EBUSY)   — see internal/atomic.h (原子 CAS)
  └─> a_spin()              — see internal/atomic.h (CPU 暂停/降耗)
```

---

## 函数规约

### 1. pthread_spin_lock

```c
int pthread_spin_lock(pthread_spinlock_t *s);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

以忙等待（自旋）方式获取自旋锁。若锁已被其他线程持有，调用线程在循环中反复检查直到锁可用，不进入内核睡眠。

#### 前置条件

- `s != NULL`，指向通过 `pthread_spin_init` 初始化的自旋锁对象
- 调用线程当前未持有该自旋锁（自旋锁不可重入）

#### 后置条件

- Case 1 锁空闲（`*s == 0`）：CAS 原子地将 `*s` 从 0 设为 `EBUSY`，返回 0，调用线程获得锁
- Case 2 锁被占用：在 while 循环中反复执行 `a_spin()` 直到 `*s == 0` 且 CAS 成功
- `a_spin()` 在每次失败后执行，通过 `pause` 指令或等效方式减少 CPU 争用
- 锁值设为 `EBUSY`（非零）而非传统 1，以便与 EAGAIN/EWOULDBLOCK 等 errno 值区分

#### 系统算法

```
pthread_spin_lock(s):
  1. while (*(volatile int *)s != 0 || a_cas(s, 0, EBUSY) != 0):
        a_spin()   // CPU 放松/暂停
  2. return 0      // 成功获取锁
```

#### 不变量

- 自旋锁为"独占访问"锁：同一时刻最多一个线程能通过 CAS 成功将 `*s` 从 0 改为非零

#### 依赖

- `a_cas(p, t, s)` — 原子比较并交换：若 `*p == t` 则 `*p = s`，返回旧值（见 `internal/atomic.h`）
- `a_spin()` — 提示 CPU 处于自旋等待中（通常为 `pause` 指令或 `__asm__ volatile` 空循环），（见 `internal/atomic.h`）
- `EBUSY` — errno 值（来自 `<errno.h>`），用作锁"已占用"标记
