# pthread_cond_broadcast.c 规约

> musl libc 广播唤醒条件变量上所有等待线程。

---

## 依赖图

```
pthread_cond_broadcast
  ├─> __private_cond_signal(c, -1)   (Internal, 定义于 pthread_cond_timedwait.c)
  ├─> __wake(&c->_c_seq, -1, 0)      (Internal, 定义于 pthread_impl.h 内联)
  └─> a_inc(&c->_c_seq)              (原子操作宏)
```

---

## 函数规约

### 1. pthread_cond_broadcast

```c
int pthread_cond_broadcast(pthread_cond_t *c);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

唤醒所有当前阻塞在条件变量 `c` 上的线程。对进程内条件变量使用内部链表机制；对进程共享条件变量使用 `_c_seq` 计数器 + futex 唤醒。

#### 前置条件

- `c != NULL`，指向有效的 `pthread_cond_t` 对象
- 条件变量已通过 `pthread_cond_init` 初始化

#### 后置条件

- **进程内（非共享）条件变量**：委托 `__private_cond_signal(c, -1)` 处理，唤醒等待链表上所有 waiter
- **进程共享条件变量**：
  - Case 1 有等待者（`_c_waiters > 0`）：递增 `_c_seq` 并调用 `__wake(&_c_seq, -1, 0)` 唤醒所有等待者，返回 `0`
  - Case 2 无等待者（`_c_waiters == 0`）：直接返回 `0`
- 始终返回 `0`

#### 系统算法

```
pthread_cond_broadcast(c):
  1. if !c->_c_shared:
        return __private_cond_signal(c, -1)   // 私有信号，n=-1 表示全部
  2. if !c->_c_waiters:                       // 无等待者，无需操作
        return 0
  3. a_inc(&c->_c_seq)                        // 原子递增序列号
  4. __wake(&c->_c_seq, -1, 0)                // futex 唤醒所有等待 _c_seq 的线程
  5. return 0
```

#### 不变量

- 进程共享模式下，`_c_seq` 递增是广播的信号，保证所有等待者被唤醒
- `__private_cond_signal` 的 `n=-1` 语义为"唤醒全部"

#### 依赖

- `pthread_cond_t` — 定义于 `<pthread.h>`，内含 `_c_shared`, `_c_waiters`, `_c_seq` 字段
- `__private_cond_signal()` — 内部函数（定义于 `pthread_cond_timedwait.c`）
- `__wake()` — 内联 futex 唤醒函数（定义于 `pthread_impl.h`）
- `a_inc()` — 原子自增操作（定义于 `atomic.h`）
