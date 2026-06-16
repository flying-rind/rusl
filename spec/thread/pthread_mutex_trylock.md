# pthread_mutex_trylock.c 规约

> musl libc pthread 互斥锁非阻塞尝试加锁。若互斥锁不可用则立即返回 EBUSY 而非阻塞等待。包含 fast/owner 两条路径和 robust list 注册逻辑。

---

## 依赖图

```
__pthread_mutex_trylock
  ├── a_cas(&m->_m_lock, 0, EBUSY)              — NORMAL 类型的 CAS 快速路径
  └── __pthread_mutex_trylock_owner(m)           — 跟踪所有权的加锁路径

__pthread_mutex_trylock_owner
  ├── __pthread_self()                           — 获取当前线程控制块
  ├── a_cas(&m->_m_lock, old, tid)               — 原子 CAS 尝试获取锁
  ├── __syscall(SYS_set_robust_list, ...)        — 首次使用 process-shared 时注册 robust list
  ├── __syscall(SYS_futex, ..., FUTEX_UNLOCK_PI) — PI 类型 spurious success 恢复
  └── robust_list 链表指针操作                   — 将 mutex 加入线程的 robust list

pthread_mutex_trylock  (weak_alias → __pthread_mutex_trylock)
```

---

## 互斥锁位域布局

`_m_lock` 字段编码锁状态和所有权：

| 位 | 掩码 | 含义 |
|----|------|------|
| 0–29 | `0x3fffffff` | 持有者 tid（正常），或 `0x3fffffff`=死锁/不可恢复 |
| 30 | `0x40000000` | 死锁标记 (EOWNERDEAD) — robust 互斥锁持有者终止 |
| 31 | `0x80000000` | 等待者标记 — 有线程在等待此锁 |

另外 `EBUSY`(16) 用作 NORMAL 类型互斥锁的"已锁定"标记值。

---

## 函数规约

### 1. __pthread_mutex_trylock / pthread_mutex_trylock

```c
int __pthread_mutex_trylock(pthread_mutex_t *m);
int pthread_mutex_trylock(pthread_mutex_t *m);  // weak_alias
```

[Visibility]:
- `__pthread_mutex_trylock`: Internal (不导出) — musl 内部实现符号
- `pthread_mutex_trylock`: User — 通过 `<pthread.h>` 对外导出（weak_alias）

#### Intent

以非阻塞方式尝试获取互斥锁。若成功则锁定互斥锁，否则立即返回 EBUSY。NORMAL 类型互斥锁走 CAS 快速路径，其他类型委托给 `__pthread_mutex_trylock_owner` 进行所有权跟踪。

#### 前置条件

- `m != NULL`，指向一个已初始化的 `pthread_mutex_t`

#### 后置条件

- Case 1 NORMAL 类型 & 锁空闲，CAS 成功：
  - `_m_lock` 从 `0` 变为 `EBUSY`(16)
  - 返回值为 `0`
- Case 2 NORMAL 类型 & 锁被占用，CAS 失败：
  - 返回值为 `EBUSY`(16)

#### 系统算法

```
__pthread_mutex_trylock(m):
  1. if (m->_m_type & 15) == PTHREAD_MUTEX_NORMAL:
       return a_cas(&m->_m_lock, 0, EBUSY) & EBUSY
  2. // 其他类型：委托给所有权跟踪路径
     return __pthread_mutex_trylock_owner(m)
```

---

### 2. __pthread_mutex_trylock_owner

```c
int __pthread_mutex_trylock_owner(pthread_mutex_t *m);
```

[Visibility]: Internal (不导出) — musl 内部实现符号

#### Intent

为跟踪所有权的互斥锁（RECURSIVE / ERRORCHECK / PI / ROBUST）实现非阻塞加锁。处理递归重入、PI spurious success 恢复、robust deadlock 检测和 robust list 注册。

#### 前置条件

- `m != NULL`

#### 后置条件

- Case 1 锁空闲且成功获取：
  - 若是首次使用 process-shared 互斥锁：调用 `SYS_set_robust_list` 注册
  - 若 `old` 非零（之前状态为 EOWNERDEAD）：返回 `EOWNERDEAD`，`_m_count = 0`
  - 否则：返回 `0`
  - 互斥锁被加入线程的 `robust_list.head` 链表
- Case 2 递归重入（RECURSIVE 类型 & own == tid）：
  - `_m_count` 递增（上限 INT_MAX，超过返回 `EAGAIN`）
  - 返回值为 `0`
- Case 3 PI 类型 spurious success：
  - 检测到 `_m_count < 0` 且 `_m_lock` 含 `0x40000000` 或 `_m_waiters`
  - 通过 `FUTEX_UNLOCK_PI` 恢复并返回错误
  - 若含 `type&4`（robust）：返回 `ENOTRECOVERABLE`
  - 否则：返回 `EBUSY`
- Case 4 锁已被拥有 & 非递归重入：
  - 返回值为 `EBUSY`
- Case 5 不可恢复状态（own == 0x3fffffff）：
  - 返回值为 `ENOTRECOVERABLE`
- Case 6 CAS 竞态失败：
  - 若 `(type&12)==12`（PI+robust）且 `_m_waiters` 非零：返回 `ENOTRECOVERABLE`
  - 否则：返回 `EBUSY`

#### 系统算法

```
__pthread_mutex_trylock_owner(m):
  1. self = __pthread_self()
     tid = self->tid
  2. old = m->_m_lock
     own = old & 0x3fffffff
  3. if own == tid:  // 递归重入
       // PI spurious success 检测
       if (type&8) && m->_m_count < 0:
         old &= 0x40000000
         m->_m_count = 0
         goto success
       if (type&3) == PTHREAD_MUTEX_RECURSIVE:
         if m->_m_count >= INT_MAX: return EAGAIN
         m->_m_count++
         return 0
  4. if own == 0x3fffffff: return ENOTRECOVERABLE
  5. if own || (old && !(type&4)): return EBUSY
  6. // process-shared: 首次注册 robust_list
     if type & 128:
       if !self->robust_list.off:
         self->robust_list.off = offset of _m_lock relative to _m_next
         __syscall(SYS_set_robust_list, &self->robust_list, 3*sizeof(long))
       if m->_m_waiters: tid |= 0x80000000
       self->robust_list.pending = &m->_m_next
  7. tid |= old & 0x40000000  // 保留 dead 标记
  8. if a_cas(&m->_m_lock, old, tid) != old:  // CAS 失败
       self->robust_list.pending = 0
       if (type&12)==12 && m->_m_waiters: return ENOTRECOVERABLE
       return EBUSY
  9. success:
     // PI spurious success 二次检查
     if (type&8) && m->_m_waiters:
       __syscall(SYS_futex, &m->_m_lock, FUTEX_UNLOCK_PI|priv)
       self->robust_list.pending = 0
       return (type&4) ? ENOTRECOVERABLE : EBUSY
 10. // 加入线程的 robust_list
     m->_m_next = self->robust_list.head
     m->_m_prev = &self->robust_list.head
     if next != &self->robust_list.head:
       next->prev = &m->_m_next    // 更新原链表头的 prev 指针
     self->robust_list.head = &m->_m_next
     self->robust_list.pending = 0
 11. if old:  // 原锁处于 EOWNERDEAD
       m->_m_count = 0
       return EOWNERDEAD
 12. return 0
```

#### 不变量

- `robust_list.pending` 在函数返回前必然被清零
- process-shared 互斥锁的 `robust_list.off` 仅在首次使用时注册一次（惰性注册）
- 成功获取后互斥锁被正确链接到线程的 robust list 链表中

#### 依赖

| 接口 | 来源 | 说明 |
|------|------|------|
| `__pthread_self()` | `pthread_impl.h` (内部) | 获取当前线程控制块 |
| `a_cas(p, t, s)` | `atomic.h` (内部) | 原子比较交换 |
| `__syscall(SYS_futex, ...)` | 内核系统调用 | PI futex 操作 |
| `__syscall(SYS_set_robust_list, ...)` | 内核系统调用 | 向内核注册 robust list |
| `FUTEX_UNLOCK_PI` (7) | `futex.h` (内部) | PI futex 解锁操作 |
| `FUTEX_PRIVATE` (128) | `futex.h` (内部) | 进程私有 futex 标志 |
| `robust_list` 结构 | `pthread_impl.h` (内部) | 健壮互斥锁链表（每线程） |
