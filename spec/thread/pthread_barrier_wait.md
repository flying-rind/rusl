# pthread_barrier_wait.c 规约

> musl libc 屏障等待函数。支持进程内（基于实例结构的自旋+阻塞协议）和进程共享（基于 futex 的复杂协议）两种模式。

---

## 依赖图

```
pthread_barrier_wait
  ├─> pshared_barrier_wait(b)          (Internal, static 本文件)
  │     ├─> a_cas(), a_store()          (原子操作)
  │     ├─> a_fetch_add()               (原子操作)
  │     ├─> __wait(), __wake()          (futex 等待/唤醒)
  │     ├─> __vm_lock()                 (Internal, SM 锁)
  │     └─> __vm_unlock()               (Internal, SM 锁)
  ├─> a_swap(), a_store(), a_fetch_add()(原子操作)
  ├─> a_spin()                          (原子自旋/PAUSE)
  ├─> __wait(), __wake()                (futex 等待/唤醒)
  └─> __syscall(SYS_futex, ...)         (系统调用)
```

---

## 内部数据结构

### struct instance (非进程共享屏障)

```c
struct instance {
    volatile int count;    // 已到达但尚未离开的线程计数
    volatile int last;     // 最后一批线程离开信号
    volatile int waiters;  // 等待 last 信号变化的 futex 等待者
    volatile int finished; // 实例所有者(首线程)的完成标志
};
```

[Visibility]: Internal (不导出) — `static` 本文件内可见，分配在栈上

#### Intent

对非进程共享屏障，第一个进入 `pthread_barrier_wait` 的线程成为"实例所有者"，在栈上分配此结构。其他线程通过屏障对象的 `_b_inst` 指针访问此结构。

---

## 函数规约

### 1. pthread_barrier_wait

```c
int pthread_barrier_wait(pthread_barrier_t *b);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

在屏障上等待直到指定数量的线程到达。最后一个到达的线程（及进程共享模式下的"序列线程"）返回 `PTHREAD_BARRIER_SERIAL_THREAD`，其余线程返回 `0`。

#### 前置条件

- `b != NULL`，指向已通过 `pthread_barrier_init` 初始化的 `pthread_barrier_t`
- 调用线程数不超过初始化时的 `count`（不应重复使用，或已在前一轮 barrier 完成后）

#### 后置条件

- Case 1 `count == 1`（`_b_limit == 0`）：立即返回 `PTHREAD_BARRIER_SERIAL_THREAD`
- Case 2 进程共享（`_b_limit < 0`）：委托 `pshared_barrier_wait`（见下文）
- Case 3 非进程共享正常流程：
  - 最后到达的 `count` 个线程中的第 `count` 个：返回 `PTHREAD_BARRIER_SERIAL_THREAD`（在实例所有者场景）或 `0`（非实例所有者）
  - 其余线程：返回 `0`

#### 系统算法 (非进程共享)

```
pthread_barrier_wait(b):
  1. limit = b->_b_limit    // count - 1

  // 平凡情况：count = 1
  2. if !limit: return PTHREAD_BARRIER_SERIAL_THREAD

  // 进程共享屏障走单独路径
  3. if limit < 0: return pshared_barrier_wait(b)

  // 获取屏障锁
  4. while a_swap(&b->_b_lock, 1):                 // 自旋获取锁
        __wait(&b->_b_lock, &b->_b_waiters, 1, 1)
  5. inst = b->_b_inst

  // --- 分支 A: 第一个到达的线程（实例所有者） ---
  6. if !inst:
        struct instance new_inst = { 0 }           // 栈上分配
        int spins = 200
        b->_b_inst = inst = &new_inst              // 发布实例
        a_store(&b->_b_lock, 0)                     // 释放锁
        if b->_b_waiters: __wake(&b->_b_lock, 1, 1)
        while spins-- && !inst->finished:           // 最多自旋 200 次
            a_spin()                                // CPU PAUSE
        a_inc(&inst->finished)                      // 标记所有者完成自旋
        while inst->finished == 1:                  // 等待所有其他线程到达并离开
            futex FUTEX_WAIT on &inst->finished     // 阻塞等待
        return PTHREAD_BARRIER_SERIAL_THREAD

  // --- 分支 B: 后续到达的线程 ---
  7. if ++inst->count == limit:                     // 最后一个到达
        b->_b_inst = 0                              // 重置实例
        a_store(&b->_b_lock, 0)                     // 释放锁
        if b->_b_waiters: __wake(&b->_b_lock, 1, 1)
        a_store(&inst->last, 1)                     // 发信号给等待的线程
        if inst->waiters: __wake(&inst->last, -1, 1)
     else:                                          // 非最后一个
        a_store(&b->_b_lock, 0)                     // 释放锁
        if b->_b_waiters: __wake(&b->_b_lock, 1, 1)
        __wait(&inst->last, &inst->waiters, 0, 1)   // 等待 last 信号

  // --- 清理: 最后一个离开的线程唤醒实例所有者 ---
  8. if a_fetch_add(&inst->count, -1) == 1          // 倒数第二个离开者
        && a_fetch_add(&inst->finished, 1):          // 递增 finished
            __wake(&inst->finished, 1, 1)            // 唤醒实例所有者

  9. return 0
```

#### 不变量 (非进程共享)

- **实例生命周期**：第一个线程分配 `instance`，最后一批线程离开前实例保持有效
- **锁协议**：对 `_b_lock` 的访问使用 `a_swap` 自旋 + `__wait` futex 等待
- **finished 字段**：从 0 到 1（所有者自旋完）到 2（所有线程离开），`futex(FUTEX_WAIT, 1)` 在 finished==1 时阻塞，被 `a_fetch_add(&inst->finished,1)` 从 1 变 2 时唤醒

---

### 2. pshared_barrier_wait

```c
static int pshared_barrier_wait(pthread_barrier_t *b);
```

[Visibility]: Internal (不导出) — `static` 函数，仅本文件内部可见

#### Intent

实现进程共享屏障的等待协议。使用基于 `_b_lock` 计数器和 futex 的多阶段协议，加上 VM 锁保证跨地址空间安全性。

#### 前置条件

- `b->_b_limit < 0`（进程共享标记）
- `limit = (b->_b_limit & INT_MAX) + 1` 为正整数

#### 后置条件

- 被选为"序列线程"的调用者返回 `PTHREAD_BARRIER_SERIAL_THREAD`
- 其余调用者返回 `0`

#### 系统算法

```
pshared_barrier_wait(b):
  limit = (b->_b_limit & INT_MAX) + 1

  if limit == 1: return PTHREAD_BARRIER_SERIAL_THREAD

  // --- 阶段 1: 获取屏障锁 ---
  while a_cas(&b->_b_lock, 0, limit) 失败:
      __wait(&b->_b_lock, &b->_b_waiters, v, 0)

  // --- 阶段 2: 等待所有线程到达 ---
  if ++b->_b_count == limit:              // 最后一个到达
      a_store(&b->_b_count, 0)
      ret = PTHREAD_BARRIER_SERIAL_THREAD
      if b->_b_waiters2: __wake(&b->_b_count, -1, 0)  // 广播唤醒
  else:
      a_store(&b->_b_lock, 0)             // 释放锁给下一个到达者
      if b->_b_waiters: __wake(&b->_b_lock, 1, 0)
      while b->_b_count > 0:              // 等待最后一个线程到达
          __wait(&b->_b_count, &b->_b_waiters2, v, 0)

  // --- 阶段 3: VM 锁同步 ---
  __vm_lock()
  if a_fetch_add(&b->_b_count, -1) == 1-limit:  // 最后一个进入 VM 锁
      a_store(&b->_b_count, 0)
      if b->_b_waiters2: __wake(&b->_b_count, -1, 0)
  else:
      while b->_b_count:                  // 等待所有线程获取 VM 锁
          __wait(&b->_b_count, &b->_b_waiters2, v, 0)

  // --- 阶段 4: 自同步销毁安全解锁 ---
  do:
      v = b->_b_lock
      w = b->_b_waiters
  while a_cas(&b->_b_lock, v, v==INT_MIN+1 ? 0 : v-1) != v

  // INT_MIN+1 表示销毁线程已设置标志，当前是最后一个退出者
  if v == INT_MIN+1 或 (v == 1 且 w):
      __wake(&b->_b_lock, 1, 0)           // 唤醒销毁线程或下一个等待者

  __vm_unlock()
  return ret
```

#### 不变量

- **_b_lock 计数协议**：初始设为 `limit`，每个线程进入锁后递减；最后一个退出者将其归零，使屏障可重用
- **VM 锁保护**：`__vm_lock()/__vm_unlock()` 确保跨进程的共享内存映射在多阶段协议间保持一致
- **_b_count 两阶段使用**：阶段 2 计数到达（递增到 limit），阶段 3 计数获取 VM 锁（递减到 0）
- **自同步销毁安全**：解锁时检测 `INT_MIN+1` 表示销毁线程已设置标志，当前是最后退出者，负责唤醒销毁线程

#### 依赖

- `pthread_barrier_t` — 定义于 `<pthread.h>`，内含 `_b_limit`, `_b_lock`, `_b_waiters`, `_b_count`, `_b_waiters2`, `_b_inst` 字段
- `struct instance` — 本文件定义的内部结构体
- `a_cas()`, `a_swap()`, `a_store()`, `a_fetch_add()` — 原子操作宏（定义于 `atomic.h`）
- `a_spin()` — 原子自旋/PAUSE 指令（定义于 `atomic.h`）
- `a_inc()` — 原子自增操作（定义于 `atomic.h`）
- `__wait()`, `__wake()` — futex 等待/唤醒（定义于 `pthread_impl.h`）
- `__syscall(SYS_futex, ...)` — Linux futex 系统调用
- `__vm_lock()`, `__vm_unlock()` — 虚拟内存锁（定义于 `vmlock.c`）
- `INT_MIN`, `INT_MAX` — 来自 `<limits.h>`
- `PTHREAD_BARRIER_SERIAL_THREAD` — 定义于 `<pthread.h>`，值为 `-1`
- `FUTEX_WAIT`, `FUTEX_PRIVATE` — Linux futex 操作码
