# pthread_cond_timedwait.c 规约

> musl libc 条件变量超时等待的核心实现。本文件包含条件变量等待的全部内部机制：waiter 链表管理、自同步销毁安全锁、以及私有信号/广播实现。

---

## 依赖图

```
pthread_cond_timedwait (weak_alias) = __pthread_cond_timedwait
  ├─> __pthread_self()                                  (Internal, pthread_impl.h)
  ├─> __pthread_testcancel()                            (Internal, pthread 取消点)
  ├─> __pthread_mutex_unlock(m)                         (Internal/User)
  ├─> __pthread_setcancelstate()                        (Internal, pthread 取消状态)
  ├─> __timedwait_cp(fut, seq, clock, ts, !shared)      (Internal, futex 取消点等待)
  ├─> pthread_mutex_lock(m)                             (User, <pthread.h>)
  ├─> lock() / unlock() / unlock_requeue()              (Internal, static inline 本文件)
  ├─> a_cas(), a_swap(), a_fetch_add(), a_store()       (原子操作宏)
  ├─> a_inc(), a_dec(), a_or()                          (原子操作宏)
  ├─> __wake(), __wait()                                (futex 内联函数)
  └─> __syscall(SYS_futex, ...)                         (系统调用)

__private_cond_signal (Internal)
  ├─> lock() / unlock()                                 (Internal, static inline 本文件)
  ├─> a_cas()                                           (原子操作)
  ├─> __wait(), __wake()                                (futex 内联函数)
  └─> unlock()                                          (释放 barrier 锁)
```

---

## 内部数据结构

### struct waiter

```c
struct waiter {
    struct waiter *prev, *next;
    volatile int state, barrier;
    volatile int *notify;
};
```

[Visibility]: Internal (不导出) — 仅在本文件及 `pthread_cond_signal.c`/`pthread_cond_broadcast.c` 内部使用

#### Intent

描述条件变量等待链表中单个等待节点的状态。Waiter 对象分配在等待线程的栈上（自动存储），通过双向链表链接。

#### 字段语义

| 字段 | 语义 |
|------|------|
| `prev`, `next` | 双向链表指针，构成 CV 上的等待队列 |
| `state` | 等待者状态：`WAITING(0)`, `SIGNALED(1)`, `LEAVING(2)` |
| `barrier` | 后继节点的屏障锁，控制唤醒顺序；本节点的 `barrier` 由前驱节点持锁 |
| `notify` | 指向引用计数器的指针，信号线程用于等待 `LEAVING` 状态的 waiter 离开队列 |

### 自同步销毁安全锁函数

```c
static inline void lock(volatile int *l);
static inline void unlock(volatile int *l);
static inline void unlock_requeue(volatile int *l, volatile int *r, int w);
```

[Visibility]: Internal (不导出) — `static inline`，仅在本文件内部可见

#### Intent

提供自同步销毁安全的自旋-等待锁实现。锁的状态值有特殊含义：
- `0`：未锁定
- `1`：已锁定，无等待者
- `2`：已锁定，有等待者

#### lock 算法

```
lock(l):
  1. if a_cas(l, 0, 1) 成功: return  // 快速路径：无竞争加锁
  2. a_cas(l, 1, 2)                  // 标记 "有等待者"
  3. do:
  4.     __wait(l, 0, 2, 1)          // futex 等待锁变为非 2
  5. while a_cas(l, 0, 2) 失败      // 尝试从 0 设为 2（持有+等待者标志）
```

#### unlock 算法

```
unlock(l):
  1. if a_swap(l, 0) == 2:           // 原子交换为 0，检查旧值
  2.     __wake(l, 1, 1)             // 旧值为 2（有等待者），唤醒一个
```

#### unlock_requeue 算法

```
unlock_requeue(l, r, w):
  1. a_store(l, 0)                   // 释放锁
  2. if w != 0:
        __wake(l, 1, 1)              // 有等待则主动唤醒一个
     else:
        尝试 FUTEX_REQUEUE|PRIVATE 将等待者迁移到锁 r, 失败则 FUTEX_REQUEUE
```

### 等待者状态枚举

```c
enum { WAITING, SIGNALED, LEAVING };
```

[Visibility]: Internal (不导出)

- `WAITING(0)`：等待者在条件变量上阻塞
- `SIGNALED(1)`：等待者已被 signal/broadcast 选中唤醒
- `LEAVING(2)`：等待者自行超时/取消离开

---

## 函数规约

### 1. __pthread_cond_timedwait

```c
int __pthread_cond_timedwait(pthread_cond_t *restrict c, pthread_mutex_t *restrict m, const struct timespec *restrict ts);
```

[Visibility]: Internal (不导出) — `__` 前缀的内部实现函数，通过 `weak_alias` 导出为 `pthread_cond_timedwait`（User 可见）

#### Intent

在条件变量上阻塞当前线程，原子释放 mutex 并进入等待，直至被 signal/broadcast 唤醒或超时到达。支持进程内（链表机制）和进程共享（futex 计数器机制）两种模式。

#### 前置条件

- `c != NULL`，指向有效 `pthread_cond_t`
- `m != NULL`，指向已由当前线程锁定的 `pthread_mutex_t`
- `ts` 可为 `NULL`（无限等待），或指向有效 `struct timespec`
- `ts->tv_nsec < 1000000000UL`（纳秒合法）
- 若 mutex 为健壮/错误检查类型（`_m_type & 15`），当前线程必须是 mutex 持有者
- 当前线程尚未持有任何取消清理处理程序

#### 后置条件

- 返回时 mutex 已由当前线程重新锁定（relock）
- Case 1 正常被 signal/broadcast 唤醒：
  - 返回 `0`
- Case 2 超时：
  - 返回 `ETIMEDOUT`
- Case 3 被取消：
  - 返回 `ECANCELED`（仅在特定条件下；若 signal 已被消费则被抑制为 `0`）
- Case 4 被信号中断：
  - 返回 `EINTR`
- Case 5 mutex 类型校验失败（错误检查/健壮锁并非当前线程持有）：
  - 返回 `EPERM`，不等待
- Case 6 `ts->tv_nsec >= 10^9`：
  - 返回 `EINVAL`

#### 系统算法

```
__pthread_cond_timedwait(c, m, ts):
  // --- 前置校验 ---
  1. if m 为错误检查/健壮锁 且 当前线程非持有者:
        return EPERM
  2. if ts 存在 且 ts->tv_nsec >= 10^9:
        return EINVAL
  3. __pthread_testcancel()         // 取消点

  // --- 注册为等待者 ---
  4. if c 为进程共享 (c->_c_shared):
        shared = 1
        fut = &c->_c_seq            // 在 CV 序列号上等待
        seq = c->_c_seq
        a_inc(&c->_c_waiters)       // 原子递增等待者计数
     else:                          // 进程内模式
        lock(&c->_c_lock)            // 获取 CV 内部锁
        seq = node.barrier = 2       // 初始化 barrier 为 2（未就绪）
        fut = &node.barrier
        node.state = WAITING
        // 将 node 插入 CV 的等待队列头部
        node.next = c->_c_head
        c->_c_head = &node
        if !c->_c_tail: c->_c_tail = &node
        else: node.next->prev = &node
        unlock(&c->_c_lock)

  // --- 释放 mutex 并等待 ---
  5. __pthread_mutex_unlock(m)
  6. 保存取消状态，若已是 DISABLE 则恢复后屏蔽取消
  7. do:
        e = __timedwait_cp(fut, seq, clock, ts, !shared)  // 取消点 futex 等待
     while *fut == seq 且 (e == 0 或 e == EINTR)          // 虚假唤醒检测
  8. if e == EINTR: e = 0                                  // EINTR 映射为成功

  // --- 离开等待状态 ---
  9. if shared:
        if e == ECANCELED 且 c->_c_seq != seq: e = 0      // 若 signal 已消费，抑制取消
        if a_fetch_add(&c->_c_waiters, -1) == -0x7fffffff: // 原子递减，检测销毁信号
            __wake(&c->_c_waiters, 1, 0)                   // 通知销毁线程
        oldstate = WAITING
        goto relock

  10. oldstate = a_cas(&node.state, WAITING, LEAVING)      // 尝试从 WAITING 转为 LEAVING

  11. if oldstate == WAITING:                               // 未被 signal/broadcast 选中
        // 从 CV 链表中移除自身
        lock(&c->_c_lock)
        if c->_c_head == &node: c->_c_head = node.next
        else if node.prev: node.prev->next = node.next
        if c->_c_tail == &node: c->_c_tail = node.prev
        else if node.next: node.next->prev = node.prev
        unlock(&c->_c_lock)
        // 若广播线程在等待本节点离开，通知它
        if node.notify:
            if a_fetch_add(node.notify, -1) == 1:
                __wake(node.notify, 1, 1)
     else:                                                  // oldstate == SIGNALED
        lock(&node.barrier)                                 // 获取屏障锁（控制唤醒顺序）

  // --- 重新锁定 mutex ---
  relock:
  12. if (tmp = pthread_mutex_lock(m)): e = tmp             // mutex 锁错误覆盖其他错误

  13. if oldstate == WAITING: goto done

  // --- 传递唤醒（barrier 链） ---
  14. if !node.next 且 mutex 非递归类型:
        a_inc(&m->_m_waiters)                               // 标记 mutex 上有等待者
  15. if node.prev:                                         // 有后继等待者
        if m->_m_lock > 0:                                 // mutex 未被竞争持有
            a_cas(&m->_m_lock, val, val|0x80000000)         // 标记 "等待者存在"
        unlock_requeue(&node.prev->barrier, &m->_m_lock, ...) // 释放前驱 barrier，传递给 mutex
     else if mutex 非递归:                                  // 链尾节点
        a_dec(&m->_m_waiters)

  // --- 取消处理 ---
  16. if e == ECANCELED: e = 0                              // signal 已消费，取消被抑制

  done:
  17. 恢复取消状态
  18. if e == ECANCELED:                                    // 取消未被抑制时
        __pthread_testcancel()
        __pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, 0)
  19. return e
```

#### 不变量

- **waiter 状态转换**：`WAITING -> SIGNALED` 或 `WAITING -> LEAVING`，不可逆，原子 CAS
- **barrier 锁链**：每个 SIGNALED 节点的 `barrier` 由前驱节点持有，传递顺序保证 FIFO 唤醒
- **引用计数协议**：`node.notify` 指向的计数器保证 `LEAVING` 状态的 waiter 从链表中移除后，signal/broadcast 线程才能返回
- **取消安全**：若 signal 已被消费（`_c_seq` 改变），取消被抑制，防止竞态条件
- **进程共享模式不使用 waiter 链表**：因为跨进程栈不可见

---

### 2. __private_cond_signal

```c
int __private_cond_signal(pthread_cond_t *c, int n);
```

[Visibility]: Internal (不导出) — `__` 前缀的内部函数，被 `pthread_cond_signal.c`（n=1）和 `pthread_cond_broadcast.c`（n=-1）调用

#### Intent

对进程内条件变量的等待链表执行 signal（n=1 唤醒一个）或 broadcast（n=-1 唤醒全部）操作。从链表尾部向头部遍历，将遇到的前 n 个 WAITING 状态的 waiter 标记为 SIGNALED；遇到 LEAVING 状态的 waiter 则通过 `notify` 引用计数等待其离开。

#### 前置条件

- `c != NULL`，指向有效的进程内条件变量
- `c->_c_shared == NULL`（非进程共享）
- `n == 1`（signal）或 `n == -1`（broadcast，唤醒全部）

#### 后置条件

- 返回 `0`
- `n > 0` 时：至多唤醒 n 个等待者
- `n < 0` 时：唤醒全部 WAITING 状态的等待者
- 任何 LEAVING 状态的 waiter 已被等待完成
- 链表从尾部被"分裂"：已标记 SIGNALED 的部分脱离 CV，剩余部分仍留在 CV 上

#### 系统算法

```
__private_cond_signal(c, n):
  1. lock(&c->_c_lock)
  2. for p = c->_c_tail; n && p; p = p->prev:
        if a_cas(&p->state, WAITING, SIGNALED) != WAITING:  // 节点在 LEAVING
            ref++
            p->notify = &ref                                // 通知该节点本函数在等待
        else:
            n--                                             // 成功标记一个
            if !first: first = p
  // 分裂链表
  3. if p:                                                  // 还有剩余节点
        if p->next: p->next->prev = 0
        p->next = 0
     else:
        c->_c_head = 0                                      // 全部被唤醒
  4. c->_c_tail = p
  5. unlock(&c->_c_lock)
  // 等待 LEAVING 状态的节点离开
  6. while ref > 0: __wait(&ref, 0, cur, 1)
  // 启动唤醒链：释放第一个 SIGNALED 节点的 barrier
  7. if first: unlock(&first->barrier)
  8. return 0
```

#### 不变量

- 链表操作在 `c->_c_lock` 保护下进行
- 唤醒顺序为 FIFO（从尾部到头部，即从最久等待者到最新等待者）
- `ref` 计数器归零确保所有 LEAVING 节点已从链表移除

#### 依赖

- `struct waiter` — 本文件定义的内部结构体
- `lock()`, `unlock()` — 本文件定义的内部锁函数
- `a_cas()` — 原子比较并交换（定义于 `atomic.h`）
- `__wait()`, `__wake()` — futex 内联函数（定义于 `pthread_impl.h`）
- `pthread_cond_t` — 定义于 `<pthread.h>`，内含 `_c_lock`, `_c_head`, `_c_tail` 字段
- `__pthread_self()` — 获取当前线程 ID（定义于 `pthread_impl.h`）
- `__pthread_mutex_unlock()` — 释放 pthread mutex
- `__timedwait_cp()` — 取消点 futex 等待（定义于内部源文件）
- `pthread_mutex_lock()` — 重新锁定 mutex

---

### 3. pthread_cond_timedwait (via weak_alias)

```c
int pthread_cond_timedwait(pthread_cond_t *restrict c, pthread_mutex_t *restrict m, const struct timespec *restrict ts);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)，是 `__pthread_cond_timedwait` 的弱别名

#### Intent

同 `__pthread_cond_timedwait`。符号 `pthread_cond_timedwait` 是用户直接调用的 POSIX 接口名称，实现与 `__pthread_cond_timedwait` 完全相同。

#### 前置/后置条件/不变量/算法

与 `__pthread_cond_timedwait` 完全相同。
