# pthread_mutex_unlock.c 规约

> musl libc pthread 互斥锁解锁。处理从线程 robust list 中摘除互斥锁、递归计数递减、以及通过 futex 唤醒阻塞等待者。

---

## 依赖图

```
__pthread_mutex_unlock
  ├── __pthread_self()                            — 获取当前线程控制块（所有权校验）
  ├── a_cas(&m->_m_lock, old, new)                — PI 类型的 CAS 解锁
  ├── a_swap(&m->_m_lock, new)                    — 普通类型的交换解锁
  ├── a_store(&m->_m_waiters, -1)                 — PI 类型标记 spurious waiters
  ├── __syscall(SYS_futex, FUTEX_UNLOCK_PI|priv)   — PI futex 解锁
  ├── __wake(&m->_m_lock, 1, priv)                — futex 唤醒等待者
  ├── __vm_lock() / __vm_unlock()                 — 进程私有锁的 VM 区域同步
  └── robust_list 链表操作                        — 从线程的 robust_list 中摘除 mutex

pthread_mutex_unlock  (weak_alias → __pthread_mutex_unlock)
```

---

## 函数规约

### 1. __pthread_mutex_unlock / pthread_mutex_unlock

```c
int __pthread_mutex_unlock(pthread_mutex_t *m);
int pthread_mutex_unlock(pthread_mutex_t *m);  // weak_alias
```

[Visibility]:
- `__pthread_mutex_unlock`: Internal (不导出) — musl 内部实现符号
- `pthread_mutex_unlock`: User — 通过 `<pthread.h>` 对外导出（weak_alias）

#### Intent

释放调用线程持有的互斥锁。处理递归计数、robust list 摘除、PI futex 解锁以及唤醒等待者。

#### 前置条件

- `m != NULL`，指向一个已初始化的 `pthread_mutex_t`
- 非 NORMAL 类型时：调用线程必须是锁的持有者

#### 后置条件

- Case 1 NORMAL 类型：
  - `_m_lock` 被设置为 0（或 new 值）
  - 若存在等待者或 `cont<0`：调用 `__wake` 唤醒一个等待者
  - 返回值为 `0`
- Case 2 RECURSIVE 类型且 `_m_count > 0`：
  - `_m_count` 递减 1，互斥锁仍被持有
  - 返回值为 `0`
- Case 3 非 NORMAL 类型且调用者非持有者：
  - 返回值为 `EPERM`
- Case 4 ROBUST 类型且原锁处于 EOWNERDEAD：
  - `new = 0x7fffffff`（标记为不可恢复）
  - 释放锁后存在等待者时唤醒
  - 返回值为 `0`
- Case 5 PI 类型（type&8）：
  - CAS 尝试设置为 new，失败时通过 PI futex 解锁
  - 返回值为 `0`

#### 系统算法

```
__pthread_mutex_unlock(m):
  1. waiters = m->_m_waiters
     type = m->_m_type & 15
     priv = (m->_m_type & 128) ^ 128
     new = 0

  2. // 所有权校验 + robust_list 摘除
     if type != PTHREAD_MUTEX_NORMAL:
       self = __pthread_self()
       old = m->_m_lock
       own = old & 0x3fffffff
       if own != self->tid: return EPERM       // 非持有者

       if (type&3) == PTHREAD_MUTEX_RECURSIVE && m->_m_count:
         return m->_m_count--, 0               // 仍有递归计数

       if (type&4) && (old & 0x40000000):      // robust deadlock
         new = 0x7fffffff                      // ENOTRECOVERABLE 标记

       if !priv:
         self->robust_list.pending = &m->_m_next
         __vm_lock()                             // 获取 VM 锁

       // 从 robust_list 链表中摘除
       prev = m->_m_prev
       next = m->_m_next
       *prev = next                              // 前驱的 next 跳过当前节点
       if next != &self->robust_list.head:
         next->prev = prev                       // 后继的 prev 指向前驱

  3. // 解锁操作
     if type & 8:  // PI 类型
       if old<0 || a_cas(&m->_m_lock, old, new) != old:
         if new: a_store(&m->_m_waiters, -1)
         __syscall(SYS_futex, &m->_m_lock, FUTEX_UNLOCK_PI|priv)
       cont = 0
       waiters = 0
     else:         // 普通类型
       cont = a_swap(&m->_m_lock, new)

  4. // 清理
     if type != PTHREAD_MUTEX_NORMAL && !priv:
       self->robust_list.pending = 0
       __vm_unlock()

  5. // 唤醒等待者
     if waiters || cont < 0:
       __wake(&m->_m_lock, 1, priv)

  6. return 0
```

#### 不变量

- 解锁后互斥锁不再存在于线程的 `robust_list` 链表中
- NORMAL 类型不维护 robust list，因此跳过所有权校验和链表操作
- `__vm_lock()` 和 `__vm_unlock()` 成对调用，保护 robust_list 链表操作的原子性
- `robust_list.pending` 在函数返回前必然被清零

#### 依赖

| 接口 | 来源 | 说明 |
|------|------|------|
| `__pthread_self()` | `pthread_impl.h` (内部) | 获取当前线程控制块 |
| `a_cas(p, t, s)` | `atomic.h` (内部) | 原子比较交换 |
| `a_swap(p, v)` | `atomic.h` (内部) | 原子交换 |
| `a_store(p, v)` | `atomic.h` (内部) | 原子写入 |
| `__syscall(SYS_futex, ...)` | 内核系统调用 | PI futex 解锁 |
| `__wake(addr, cnt, priv)` | `pthread_impl.h` (内部) | futex 唤醒等待者 |
| `__vm_lock()` / `__vm_unlock()` | `pthread_impl.h` (内部) | 虚拟内存区域锁（保护 robust list） |
| `FUTEX_UNLOCK_PI` (7) | `futex.h` (内部) | PI futex 解锁操作 |
| `FUTEX_PRIVATE` (128) | `futex.h` (内部) | 进程私有 futex |

#### 与 trylock / timedlock 的协作关系

unlock 中的 `prev = m->_m_prev; next = m->_m_next; *prev = next` 链表摘除操作对应 trylock 中 `m->_m_next = self->robust_list.head` 的链表插入操作。两者配合维护 per-thread robust list 的正确性。
