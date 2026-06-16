# pthread_cond_destroy.c 规约

> musl libc 销毁条件变量。

---

## 依赖图

```
pthread_cond_destroy
  ├─> a_or(&c->_c_waiters, 0x80000000)       (原子操作)
  ├─> a_inc(&c->_c_seq)                       (原子操作)
  ├─> __wake(&c->_c_seq, -1, 0)               (futex 唤醒)
  └─> __wait(&c->_c_waiters, 0, cnt, 0)       (futex 等待)
```

---

## 函数规约

### 1. pthread_cond_destroy

```c
int pthread_cond_destroy(pthread_cond_t *c);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

销毁条件变量。对于进程共享的条件变量，如果有等待者正在等待，则广播唤醒所有等待者并阻塞直到它们全部退出等待状态，确保安全销毁。

#### 前置条件

- `c != NULL`，指向有效的 `pthread_cond_t` 对象
- 无任何线程正在调用 `pthread_cond_wait` 或 `pthread_cond_timedwait` 使用该条件变量（除了当前正在等待且即将被销毁过程唤醒的线程）

#### 后置条件

- **非进程共享条件变量**（`_c_shared == NULL`）：立即返回 `0`（依赖调用者保证无等待者）
- **进程共享条件变量**（`_c_shared == (void*)-1`）：
  - Case 1 有等待者（`_c_waiters > 0`）：
    - 在 `_c_waiters` 上设置标志位 0x80000000 标记"销毁中"
    - 递增 `_c_seq` 并 futex 广播唤醒所有等待者
    - 忙等直到 `_c_waiters` 低 31 位降到 0（所有等待者已退出）
    - 返回 `0`
  - Case 2 无等待者：立即返回 `0`
- 始终返回 `0`

#### 系统算法

```
pthread_cond_destroy(c):
  1. if c->_c_shared 且有等待者:
  2.     a_or(&c->_c_waiters, 0x80000000)   // 标记 "销毁中"，等待者看到后会加速退出
  3.     a_inc(&c->_c_seq)                    // 递增序列号以唤醒所有等待者
  4.     __wake(&c->_c_seq, -1, 0)            // futex 广播唤醒
  5.     while (c->_c_waiters & 0x7fffffff):  // 等待所有等待者退出
  6.         __wait(&c->_c_waiters, 0, cnt, 0)  // futex 等待 _c_waiters 变化
  7. return 0
```

#### 不变量

- 销毁过程必须等待所有正在等待的线程安全退出后才返回
- `_c_waiters` 的 0x80000000 位充当销毁信号，等待者线程在 `pthread_cond_timedwait` 中通过 `a_fetch_add(&c->_c_waiters, -1) == -0x7fffffff` 检测到销毁并加速清理

#### 依赖

- `pthread_cond_t` — 定义于 `<pthread.h>`，内含 `_c_shared`, `_c_waiters`, `_c_seq` 字段
- `a_or()` — 原子或操作（定义于 `atomic.h`）
- `a_inc()` — 原子自增操作（定义于 `atomic.h`）
- `__wake()` — 内联 futex 唤醒函数（定义于 `pthread_impl.h`）
- `__wait()` — 内联 futex 等待函数（定义于 `pthread_impl.h`）
