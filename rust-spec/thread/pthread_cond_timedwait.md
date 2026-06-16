# pthread_cond_timedwait -- Rust 接口归约

> 本文件是条件变量等待的核心实现模块，包含 `__pthread_cond_timedwait`、`pthread_cond_timedwait`（weak_alias）和 `__private_cond_signal` 三个对外导出函数的归约，以及内部 waiter 链表和自同步锁的架构设计。

---

## 原始 C 接口

```c
// 主实现（内部符号，通过 weak_alias 导出为 pthread_cond_timedwait）
int __pthread_cond_timedwait(pthread_cond_t *restrict c, pthread_mutex_t *restrict m, const struct timespec *restrict ts);

// POSIX 接口（__pthread_cond_timedwait 的弱别名）
int pthread_cond_timedwait(pthread_cond_t *restrict c, pthread_mutex_t *restrict m, const struct timespec *restrict ts);

// 内部信号函数（被 pthread_cond_signal / pthread_cond_broadcast 调用）
int __private_cond_signal(pthread_cond_t *c, int n);
```

---

## Rust 外部 ABI 接口

```rust
// musl __ 前缀主实现，必须对外导出
extern "C" fn __pthread_cond_timedwait(
    c: *mut pthread_cond_t,
    m: *mut pthread_mutex_t,
    ts: *const timespec,
) -> core::ffi::c_int;

// POSIX 用户接口，与 __pthread_cond_timedwait 实现相同
extern "C" fn pthread_cond_timedwait(
    c: *mut pthread_cond_t,
    m: *mut pthread_mutex_t,
    ts: *const timespec,
) -> core::ffi::c_int;

// 内部信号函数，被 cond_signal/cond_broadcast 调用（musl 文件内 static，但需对外导出）
extern "C" fn __private_cond_signal(
    c: *mut pthread_cond_t,
    n: core::ffi::c_int,
) -> core::ffi::c_int;
```

---

## 内部数据结构（Rust 安全抽象重设计）

原 C 实现使用 C 风格手写双向链表。Rust 版本可使用 Rust 所有权和安全抽象重设计内部结构。

### struct Waiter

```rust
// 模块内部可见（pub(crate)），不对外暴露
pub(crate) struct Waiter {
    prev: Cell<*mut Waiter>,
    next: Cell<*mut Waiter>,
    state: AtomicI32,      // WAITING(0) | SIGNALED(1) | LEAVING(2)
    barrier: AtomicI32,     // 屏障锁，初始 = 2（未就绪），由前驱唤醒
    notify: Cell<*mut AtomicI32>,  // 指向引用计数器的指针
}
```

[Visibility]: Internal — `pub(crate)`，仅在 cond 相关模块间共享

| 字段 | Rust 类型 | 语义 |
|------|-----------|------|
| `prev`, `next` | `Cell<*mut Waiter>` | 双向链表指针，构成 CV 上的等待队列 |
| `state` | `AtomicI32` | 等待者状态：`WAITING(0)`, `SIGNALED(1)`, `LEAVING(2)` |
| `barrier` | `AtomicI32` | 后继节点的屏障锁；本节点的 barrier 由前驱持锁 |
| `notify` | `Cell<*mut AtomicI32>` | 指向引用计数器的指针，信号线程用于等待 LEAVING 状态的 waiter 离开队列 |

**Rust 设计说明**：`Waiter` 分配在等待线程的栈上（自动存储），因此使用裸指针和 `Cell` 管理链表关系。`prev`/`next` 使用 `Cell<*mut Waiter>` 替代 C 的普通指针，因为链表跨线程共享且通过锁同步。

### 等待者状态枚举

```rust
// 模块内部可见
pub(crate) enum WaiterState {
    Waiting = 0,   // 等待者在条件变量上阻塞
    Signaled = 1,  // 等待者已被 signal/broadcast 选中唤醒
    Leaving = 2,   // 等待者自行超时/取消离开
}
```

[Visibility]: Internal — `pub(crate)`

---

### 自同步销毁安全锁函数（Rust 重设计）

原 C 使用 `static inline` 函数操作 `volatile int *l`，实现自旋-等待的自同步锁。Rust 版本可将其封装为类型安全的锁抽象：

```rust
/// 自同步销毁安全的自旋-等待锁
///
/// 锁状态值：
/// - 0: 未锁定
/// - 1: 已锁定，无等待者
/// - 2: 已锁定，有等待者
pub(crate) struct SelfSyncLock {
    val: AtomicI32,
}
```

```rust
impl SelfSyncLock {
    /// 创建未锁定的自同步锁
    pub(crate) const fn new() -> SelfSyncLock;

    /// 加锁
    ///
    /// 算法：
    /// 1. 快速路径：compare_exchange(0, 1) 成功则立即返回
    /// 2. 若 compare_exchange(1, 2) 成功：标记"有等待者"
    /// 3. futex wait 循环：直到 compare_exchange(0, 2) 成功
    pub(crate) fn lock(&self);

    /// 解锁
    ///
    /// 算法：
    /// 1. swap(0) 获取旧值
    /// 2. 若旧值为 2（有等待者）：futex wake 一个等待者
    pub(crate) fn unlock(&self);

    /// 解锁并将等待者迁移到目标锁
    ///
    /// 算法：
    /// 1. store(0) 释放锁
    /// 2. 若有等待者（w != 0）：futex wake 一个
    /// 3. 否则：尝试 FUTEX_REQUEUE | FUTEX_PRIVATE 将等待者迁移到 r
    pub(crate) fn unlock_requeue(&self, r: &AtomicI32, w: c_int);
}
```

[Visibility]: Internal — `pub(crate)`，仅在 cond 相关模块间共享

**Rust 设计说明**：将 C 的 `volatile int` + 手动原子操作封装为 `SelfSyncLock` 类型，提供 `lock()/unlock()/unlock_requeue()` 方法。内部使用 `AtomicI32` + `Ordering` 替代 C 的 `a_cas`/`a_swap` 宏。

---

## 函数规约

### 1. __pthread_cond_timedwait / pthread_cond_timedwait

```rust
extern "C" fn __pthread_cond_timedwait(
    c: *mut pthread_cond_t,
    m: *mut pthread_mutex_t,
    ts: *const timespec,
) -> core::ffi::c_int;

extern "C" fn pthread_cond_timedwait(
    c: *mut pthread_cond_t,
    m: *mut pthread_mutex_t,
    ts: *const timespec,
) -> core::ffi::c_int;
```

[Visibility]: `__pthread_cond_timedwait` — Internal (不导出，但 `__` 前缀 musl 内部符号需对外导出)；`pthread_cond_timedwait` — User，通过 `<pthread.h>` 对外导出 (POSIX)。两者实现完全相同。

#### Intent
在条件变量上阻塞当前线程，原子释放 mutex 并进入等待，直至被 signal/broadcast 唤醒或超时到达。支持进程内（链表机制）和进程共享（futex 计数器机制）两种模式。

#### 前置条件
- `c` 非空，指向有效 `pthread_cond_t`
- `m` 非空，指向已由当前线程锁定的 `pthread_mutex_t`
- `ts` 可为 `NULL`（无限等待），或指向有效 `timespec`
- `ts.tv_nsec < 1_000_000_000`（纳秒合法）
- 若 mutex 为健壮/错误检查类型（`_m_type & 15`），当前线程必须是 mutex 持有者

#### 后置条件
- 返回时 mutex 已由当前线程重新锁定（relock）
- Case 1 正常被唤醒：返回 `0`
- Case 2 超时：返回 `ETIMEDOUT`
- Case 3 被取消（且 signal 未被消费）：返回 `ECANCELED`
- Case 4 被信号中断：返回 `EINTR`
- Case 5 mutex 类型校验失败（错误检查/健壮锁并非当前线程持有）：返回 `EPERM`
- Case 6 `ts.tv_nsec >= 1_000_000_000`：返回 `EINVAL`

#### 不变量
- **Waiter 状态转换**：`WAITING -> SIGNALED` 或 `WAITING -> LEAVING`，不可逆，原子 CAS
- **Barrier 锁链**：每个 SIGNALED 节点的 `barrier` 由前驱节点持有，传递顺序保证 FIFO 唤醒
- **引用计数协议**：`node.notify` 指向的计数器保证 LEAVING 状态的 waiter 从链表中移除后，signal/broadcast 线程才能返回
- **取消安全**：若 signal 已被消费（`_c_seq` 改变），取消被抑制，防止竞态条件
- **进程共享模式不使用 waiter 链表**：因为跨进程栈不可见

#### 算法（高层概述）
```
__pthread_cond_timedwait(c, m, ts):
  1. 前置校验：mutex 所有权、ts.tv_nsec 合法性、取消点检查
  2. 注册等待者：
     - 进程共享：a_inc(&c._c_waiters)，记录 _c_seq 初始值
     - 进程内：分配栈上 Waiter，插入 CV 等待链表头部
  3. 释放 mutex，进入 timedwait_cp futex 等待（取消点）
  4. 虚假唤醒检测：若 fut 值未变 且 (e==0 或 e==EINTR)，重试等待
  5. EINTR 映射为成功（e=0）
  6. 离开等待状态：
     - 进程共享：a_fetch_add(&c._c_waiters, -1)，检测销毁信号
     - 进程内：CAS state WAITING->LEAVING；若 SIGNALED 则获取 barrier 锁
  7. 若为 LEAVING（未被 signal）：从链表移除自身；通知 notify 引用计数
  8. 重新锁定 mutex
  9. 传递唤醒（barrier 链）：释放前驱 barrier（若存在），传递给 mutex
  10. 取消处理：恢复取消状态，执行 testcancel（若取消未被抑制）
  11. 返回 e
```

---

### 2. __private_cond_signal

```rust
extern "C" fn __private_cond_signal(
    c: *mut pthread_cond_t,
    n: core::ffi::c_int,
) -> core::ffi::c_int;
```

[Visibility]: Internal (不导出，但 `__` 前缀 musl 内部符号需对外导出)。被 `pthread_cond_signal`（n=1）和 `pthread_cond_broadcast`（n=-1）调用。

#### Intent
对进程内条件变量的等待链表执行 signal（n=1 唤醒一个）或 broadcast（n=-1 唤醒全部）操作。从链表尾部向头部遍历，将遇到的前 n 个 WAITING 状态的 waiter 标记为 SIGNALED；遇到 LEAVING 状态的 waiter 则通过 `notify` 引用计数等待其离开。

#### 前置条件
- `c` 非空，指向有效的进程内条件变量
- `c._c_shared == NULL`（非进程共享）
- `n == 1`（signal）或 `n == -1`（broadcast，唤醒全部）

#### 后置条件
- 返回 `0`
- `n > 0` 时：至多唤醒 n 个等待者
- `n < 0` 时：唤醒全部 WAITING 状态的等待者
- 任何 LEAVING 状态的 waiter 已被等待完成
- 链表从尾部被"分裂"：已标记 SIGNALED 的部分脱离 CV，剩余部分仍留在 CV 上

#### 不变量
- 链表操作在 `c._c_lock` 保护下进行
- 唤醒顺序为 FIFO（从尾部到头部，即从最久等待者到最新等待者）
- `ref` 计数器归零确保所有 LEAVING 节点已从链表移除

#### 算法（高层概述）
```
__private_cond_signal(c, n):
  1. lock(&c._c_lock)
  2. 从 c._c_tail 向头部遍历：
     - 若 CAS state WAITING->SIGNALED 失败（节点在 LEAVING）：ref++，设置 notify 指针
     - 否则：n--，记录 first 指针
  3. 分裂链表：截断 first 成为新表头
  4. unlock(&c._c_lock)
  5. 等待所有 LEAVING 节点（ref 归零）
  6. 若 first：unlock(&first.barrier) 启动唤醒链
  7. return 0
```

---

## Rust 内部设计要点

### 进程内等待机制
- 使用 `SelfSyncLock` 类型封装 CV 的 `_c_lock` 字段，提供安全的自同步锁语义
- Waiter 链表操作在 `SelfSyncLock` 保护下进行，加锁/解锁边界清晰
- `barrier` 字段使用 `AtomicI32`，唤醒传递链中前驱释放后继的 barrier 锁

### 进程共享等待机制
- 不使用 Waiter 链表（跨进程栈不可见）
- 通过 `_c_seq: AtomicI32` + `_c_waiters: AtomicI32` 实现 futex 协议
- `timedwait_cp` 为取消点 futex 等待函数，定义于 `pthread_impl` 模块

### 与 C 原型的区别
- `SelfSyncLock` 封装了原 C 的 `static inline lock()/unlock()/unlock_requeue()` 三个函数
- `Waiter` 结构体使用 `Cell<*mut Waiter>` 管理链表指针，替代 C 的裸指针
- `WaiterState` 枚举替代 C 的匿名 `enum { WAITING, SIGNALED, LEAVING }`
- 原子操作使用 `core::sync::atomic` 的 `Ordering` 参数替代 C 宏的隐式内存序

---

/* Rely */
[RELY]
Predefined Types:
  pthread_cond_t                   // #[repr(C)] 条件变量类型
  pthread_mutex_t                  // #[repr(C)] 互斥锁类型
  timespec                         // #[repr(C)] 时间规格结构体
  core::ffi::c_int / c_void       // Rust 核心库 C FFI 类型

Internal Module:
  pthread_impl::PthreadCond        // 字段访问（c_seq, c_waiters, c_clock, c_lock, c_head, c_tail）
  pthread_impl::PthreadMutex       // 字段访问（m_type, m_lock, m_waiters, m_count 等）
  pthread_impl::timedwait_cp       // 取消点 futex 等待
  pthread_impl::wake / __wait      // futex 唤醒/等待
  pthread_impl::__pthread_self     // 获取当前线程 TCB
  pthread_impl::testcancel         // 线程取消点检查
  pthread_impl::setcancelstate     // 设置取消状态

External Functions (同一 crate):
  pthread_mutex_lock               // 重新锁定 mutex
  __pthread_mutex_unlock           // 释放 pthread mutex

Rust Core:
  core::sync::atomic::{AtomicI32, Ordering}  // 原子操作
  core::cell::Cell                 // 内部可变性，用于链表指针
  core::ptr                        // 裸指针操作

[GUARANTEE]
Exported Interface:
  extern "C" fn __pthread_cond_timedwait(c: *mut pthread_cond_t, m: *mut pthread_mutex_t, ts: *const timespec) -> c_int;
                                   // musl 内部符号，需对外导出
  extern "C" fn pthread_cond_timedwait(c: *mut pthread_cond_t, m: *mut pthread_mutex_t, ts: *const timespec) -> c_int;
                                   // POSIX 用户接口
  extern "C" fn __private_cond_signal(c: *mut pthread_cond_t, n: c_int) -> c_int;
                                   // 内部信号函数，被 cond_signal/cond_broadcast 调用

Internal Interface (pub(crate)):
  struct Waiter                    // 等待者链表节点
  enum WaiterState                 // 等待者状态枚举
  struct SelfSyncLock              // 自同步销毁安全锁
  impl SelfSyncLock::{lock, unlock, unlock_requeue}  // 锁操作
