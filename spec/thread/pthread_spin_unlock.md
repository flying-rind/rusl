# pthread_spin_unlock.c 规约

> musl libc 的自旋锁解锁函数实现。使用原子存储操作将自旋锁值复位为 0。

---

## 依赖图

```
pthread_spin_unlock
  └─> a_store(s, 0)  — see internal/atomic.h (原子存储)
```

---

## 函数规约

### 1. pthread_spin_unlock

```c
int pthread_spin_unlock(pthread_spinlock_t *s);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

释放自旋锁，使其可被其他等待线程获取。使用原子写操作保证对锁状态的修改对其他线程可见。

#### 前置条件

- `s != NULL`，指向通过 `pthread_spin_init` 初始化的自旋锁对象
- 调用线程当前持有该自旋锁（即此前已通过 `pthread_spin_lock` 或 `pthread_spin_trylock` 成功获取）

#### 后置条件

- `*s = 0`（解锁状态），通过原子存储写入以保证内存可见性
- 返回 0（成功）
- 其他自旋等待此锁的线程可以观察到锁变为空闲

#### 系统算法

```
pthread_spin_unlock(s):
  1. a_store(s, 0)  // 原子地将 *s 设为 0, 释放锁
  2. return 0       // 成功
```

#### 不变量

- 解锁操作总是成功，不会失败

#### 依赖

- `a_store(p, v)` — 原子存储：将 `v` 写入 `*p`，具有完整内存屏障语义（见 `internal/atomic.h`）
