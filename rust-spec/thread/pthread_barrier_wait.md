# pthread_barrier_wait -- Rust 接口归约

> 本文件是屏障等待的核心实现模块，包含 `pthread_barrier_wait` 对外导出函数以及内部 `pshared_barrier_wait` 辅助函数的归约。

---

## 原始 C 接口

```c
// POSIX 屏障等待接口
int pthread_barrier_wait(pthread_barrier_t *b);

// 内部静态函数（进程共享屏障等待协议）
static int pshared_barrier_wait(pthread_barrier_t *b);
```

---

## Rust 外部 ABI 接口

```rust
// POSIX 用户接口
extern "C" fn pthread_barrier_wait(b: *mut pthread_barrier_t) -> core::ffi::c_int;
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

---

## 内部数据结构（Rust 安全抽象重设计）

### struct Instance (非进程共享屏障)

```rust
/// 非进程共享屏障的实例状态结构体
///
/// 第一个进入 pthread_barrier_wait 的线程成为"实例所有者"，
/// 在栈上分配此结构。其他线程通过屏障对象的 _b_inst 指针访问。
pub(crate) struct Instance {
    count: AtomicI32,    // 已到达但尚未离开的线程计数
    last: AtomicI32,     // 最后一批线程离开信号
    waiters: AtomicI32,  // 等待 last 信号变化的 futex 等待者
    finished: AtomicI32, // 实例所有者(首线程)的完成标志
}
```

[Visibility]: Internal — `pub(crate)`，仅 barrier 模块内部可见

| 字段 | Rust 类型 | 语义 |
|------|-----------|------|
| `count` | `AtomicI32` | 已到达但尚未离开的线程计数，替代 C 的 `volatile int` |
| `last` | `AtomicI32` | 最后一批线程离开信号，替代 C 的 `volatile int` |
| `waiters` | `AtomicI32` | 等待 last 信号变化的 futex 等待者 |
| `finished` | `AtomicI32` | 实例所有者的完成标志：0->1（自旋完）->2（所有线程离开） |

---

## 函数规约

### 1. pthread_barrier_wait

```rust
extern "C" fn pthread_barrier_wait(b: *mut pthread_barrier_t) -> core::ffi::c_int;
```

#### Intent
在屏障上等待直到指定数量的线程到达。最后一个到达的线程（及进程共享模式下的"序列线程"）返回 `PTHREAD_BARRIER_SERIAL_THREAD`（值为 `-1`），其余线程返回 `0`。

#### 前置条件
- `b` 非空，指向已通过 `pthread_barrier_init` 初始化的 `pthread_barrier_t`
- 调用线程数不超过初始化时的 `count`（不应重复使用，或已在前一轮 barrier 完成后）

#### 后置条件
- Case 1 `count == 1`（`_b_limit == 0`）：立即返回 `PTHREAD_BARRIER_SERIAL_THREAD`
- Case 2 进程共享（`_b_limit < 0`）：委托 `pshared_barrier_wait`（见下文）
- Case 3 非进程共享正常流程：
  - 最后到达的线程（第 count 个）：返回 `PTHREAD_BARRIER_SERIAL_THREAD`（实例所有者）或 `0`（非实例所有者场景）
  - 其余线程：返回 `0`

#### 不变量 (非进程共享)
- **实例生命周期**：第一个线程分配 `Instance`，最后一批线程离开前实例保持有效
- **锁协议**：对 `_b_lock` 的访问使用 `swap` 自旋 + futex 等待
- **finished 字段**：0 -> 1（所有者自旋完）-> 2（所有线程离开）

#### 算法（非进程共享，高层概述）
```
pthread_barrier_wait(b):
  1. limit = b._b_limit
  2. 若 limit == 0（count=1）：立即返回 PTHREAD_BARRIER_SERIAL_THREAD
  3. 若 limit < 0（进程共享）：委托 pshared_barrier_wait(b)
  4. 获取屏障锁：swap(&b._b_lock, 1)，失败则 futex 等待
  5. 若 inst 为空（首个到达者）：
     - 在栈上分配 Instance { zeros }
     - 发布 inst 指针到 b._b_inst
     - 释放锁，自旋最多 200 次等待 finished 变化
     - 标记 finished = 1（自旋完成）
     - futex 等待 finished 变为 2（所有线程离开）
     - 返回 PTHREAD_BARRIER_SERIAL_THREAD
  6. 若 ++inst.count == limit（最后一个到达）：
     - 重置 b._b_inst = null
     - 释放锁
     - 设置 inst.last = 1，广播唤醒等待者
  7. 否则（非最后一个）：
     - 释放锁
     - futex 等待 inst.last 变为非 0
  8. 清理：若倒数第二个离开（a_fetch_add(&inst.count, -1) == 1）
     - a_fetch_add(&inst.finished, 1) == 1 则唤醒所有者
  9. 返回 0
```

---

### 2. pshared_barrier_wait（内部函数，Rust 重设计）

```rust
pub(crate) fn pshared_barrier_wait(b: *mut pthread_barrier_t) -> core::ffi::c_int;
```

[Visibility]: Internal — `pub(crate)`，仅本模块内部可见

#### Intent
实现进程共享屏障的等待协议。使用基于 `_b_lock` 计数器和 futex 的多阶段协议，加上 VM 锁保证跨地址空间安全性。

#### 前置条件
- `b._b_limit < 0`（进程共享标记）
- `limit = (b._b_limit & i32::MAX) + 1` 为正整数

#### 后置条件
- 被选为"序列线程"的调用者返回 `PTHREAD_BARRIER_SERIAL_THREAD`（`-1`）
- 其余调用者返回 `0`

#### 不变量
- **_b_lock 计数协议**：初始设为 `limit`，每个线程进入锁后递减；最后一个退出者将其归零，使屏障可重用
- **VM 锁保护**：`__vm_lock() / __vm_unlock()` 确保跨进程的共享内存映射在多阶段协议间保持一致
- **_b_count 两阶段使用**：阶段 2 计数到达（递增到 limit），阶段 3 计数获取 VM 锁（递减到 0）
- **自同步销毁安全**：解锁时检测 `INT_MIN + 1`，当前线程是最后退出者时唤醒销毁线程

#### 算法（高层概述）
```
pshared_barrier_wait(b):
  limit = (b._b_limit & INT_MAX) + 1
  if limit == 1: return PTHREAD_BARRIER_SERIAL_THREAD

  阶段 1 — 获取屏障锁:
    while CAS(&b._b_lock, 0, limit) 失败: futex 等待

  阶段 2 — 等待所有线程到达:
    if ++b._b_count == limit:
        b._b_count = 0; ret = SERIAL_THREAD
        广播唤醒等待者
    else:
        释放 _b_lock; futex 等待 _b_count > 0

  阶段 3 — VM 锁同步:
    __vm_lock()
    if a_fetch_add(&b._b_count, -1) == 1-limit:
        b._b_count = 0; 广播唤醒
    else:
        等待 _b_count 变为 0

  阶段 4 — 自同步销毁安全解锁:
    CAS 循环递减 b._b_lock（若 v == INT_MIN+1 则归零）
    若最后一个退出 或 (v==1 且有等待者): futex 唤醒
    __vm_unlock()
    return ret
```

---

## Rust 内部设计要点

### 非进程共享模式
- `Instance` 结构体使用 `AtomicI32` 替代 C 的 `volatile int`，提供显式内存序
- 第一个线程在栈上分配 `Instance` 成为"实例所有者"，Rust 的借用规则保证在 `finished` 变为 2 前实例不会被释放
- `swap` 自旋 + futex 等待实现屏障锁（`_b_lock`），替代 C 的 `a_swap` + `__wait`

### 进程共享模式
- `pshared_barrier_wait` 为 `pub(crate)` 函数，仅在 barrier 模块内部可见
- 多阶段 futex 协议：lock -> arrival -> vm_sync -> unlock
- `__vm_lock() / __vm_unlock()` 确保跨地址空间安全
- 自同步销毁安全通过在 `_b_lock` 上检测 `INT_MIN + 1` 实现

### 与 C 原型的区别
- `Instance` 使用 `AtomicI32` + `Ordering` 替代 `volatile int`
- `pshared_barrier_wait` 在 Rust 中可为独立的 `pub(crate) fn`，不强制 `static`
- 原子操作使用 `compare_exchange` 替代 C 的 `a_cas` 宏

---

/* Rely */
[RELY]
Predefined Types:
  pthread_barrier_t                // #[repr(C)] 屏障类型
  core::ffi::c_int                // Rust 核心库 C FFI 类型

Internal Module:
  pthread_impl::PthreadBarrier     // 字段访问方法（b_lock, b_waiters, b_count, b_waiters2, b_limit, b_inst）
  pthread_impl::__wait / wake      // futex 等待/唤醒
  pthread_impl::__vm_lock / __vm_unlock  // VM 锁，确保共享内存安全

Rust Core:
  core::sync::atomic::{AtomicI32, Ordering}
                                   // 原子操作

Predefined Constants:
  PTHREAD_BARRIER_SERIAL_THREAD    // = -1，定义于 <pthread.h>

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_barrier_wait(b: *mut pthread_barrier_t) -> c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的符号

Internal Interface (pub(crate)):
  struct Instance                  // 非进程共享屏障的实例状态
  fn pshared_barrier_wait(b: *mut pthread_barrier_t) -> c_int;
                                   // 进程共享屏障等待协议实现
