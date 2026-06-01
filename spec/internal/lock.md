# lock.h 规约

> **来源文件**: `musl/src/internal/lock.h`
> **复杂度层级**: Level 1 — 简单模块
> **依赖图**: 无内部依赖，仅依赖编译器内建类型

---

## 概述

`lock.h` 定义了 musl 中轻量级自旋锁（spinlock）的接口，用于保护内部共享数据结构的临界区。该锁基于 `volatile int *` 上的原子操作实现，所有符号均为 musl 内部使用。

**不变量 (Invariants)**：
- **I1**: 锁变量只有两种语义状态：0 表示未锁定（unlocked），非零表示已被某一线程持有（locked）。
- **I2**: 任何时刻最多只有一个线程能成功获取同一把锁（互斥性）。
- **I3**: 只有成功获取了锁的线程才能释放该锁。

---

## 类型定义

### `volatile int *` — 锁变量类型

```
typedef volatile int* lock_t;  // 隐式约定，未显式 typedef
```

[Visibility]: Internal — musl 内部自旋锁约定类型，POSIX/C 标准未定义

musl 中锁变量被声明为 `volatile int[1]` 数组，通过指针传递，如 `volatile int __lock[1]`。`volatile` 修饰保证每次访问都从内存直接读写（抑制编译器优化重排）；`int[1]` 数组形式允许 `&lock` 自然退化为指针，同时 `sizeof(lock)` 保留原始语义。

**有效状态**：
- `0` — 未锁定
- 非零 — 已锁定（具体值是实现细节）

---

## 函数声明

### `void __lock(volatile int *)`

```c
void __lock(volatile int *);
```

[Visibility]: Internal — musl 内部自旋锁获取，POSIX/C 标准未定义

**意图 (Intent)**：
在高竞争场景下使用原子 CAS 自旋等待，避免系统调用开销。用于保护持有时间极短的临界区（通常仅数条指令）。

**前置条件 (Preconditions)**：
- **P1**: `ptr` 非空，指向一个合法的 `volatile int` 锁变量。
- **P2**: 调用者未持有该锁（禁止同一线程递归加锁）。

**后置条件 (Postconditions)**：
- **Case 1（成功获取）**：
  - **Q1**: 函数返回。
  - **Q2**: `*ptr` 被设置为非零值（锁被标记为已持有）。
  - **Q3**: 调用者进入临界区，互斥地访问受该锁保护的任何共享资源。
- **Case 2（竞争）**：
  - 函数不会立即返回，而在内部自旋循环（spin-loop）中反复尝试 CAS 操作，直到成功获取锁。此过程可能无界等待。

**系统算法 (System Algorithm)**：
使用原子 compare-and-swap (CAS) 自旋锁：
```
while (a_cas(ptr, 0, 1) != 0) a_spin();
```
即在循环中反复尝试将 `*ptr` 由 0 改为 1：若旧值为 0（锁空闲），则操作成功并返回；若旧值非零（被他人持有），则执行 `a_spin()`（通常等同于 `a_barrier()` 或一条 PAUSE 指令），然后重试。

**注意事项**：
- 不应在持有自旋锁时调用可能阻塞的函数（如 `__futexwait`、`malloc` 等），以免造成死锁或长延迟。
- 不保证公平性（FIFO 唤醒），先到不一定先得。

---

### `void __unlock(volatile int *)`

```c
void __unlock(volatile int *);
```

[Visibility]: Internal — musl 内部自旋锁释放，POSIX/C 标准未定义

**意图 (Intent)**：
原子地将锁变量置零，释放临界区，允许其他等待线程进入。

**前置条件 (Preconditions)**：
- **P1**: `ptr` 非空，指向一个已被当前线程持有的锁变量（即调用者必须已通过 `__lock(ptr)` 成功获取过该锁）。
- **P2**: 锁的持有者与解锁者必须是同一线程（或同一执行上下文）。

**后置条件 (Postconditions)**：
- **Q1**: `*ptr` 被原子地设置为 0（未锁定状态）。
- **Q2**: 调用者退出临界区，任何对该锁的并发访问限制解除。
- **Q3**: 函数无返回值（void）。

**系统算法 (System Algorithm)**：
使用原子 store 操作将锁清零：
```
a_store(ptr, 0);
```
其中 `a_store` 通常会包含完整的内存屏障（`a_barrier`），确保临界区内的所有内存写入在解锁前全局可见。

---

## 宏定义

### `LOCK(x)`

```c
#define LOCK(x) __lock(x)
```

[Visibility]: Internal — musl 内部便捷宏

对 `__lock()` 的直接包装，无额外语义。

---

### `UNLOCK(x)`

```c
#define UNLOCK(x) __unlock(x)
```

[Visibility]: Internal — musl 内部便捷宏

对 `__unlock()` 的直接包装，无额外语义。

---

## 跨文件依赖

| 依赖符号 | 来源 | 处理方式 |
|---------|------|---------|
| `a_cas()`, `a_spin()`, `a_store()`, `a_barrier()` | `atomic.h`（musl 内部，`src/internal/atomic.h`） | 跨文件依赖，该头文件提供架构相关的原子操作 |
| `volatile int` | C 语言内建类型 | 无需追踪 |

---

## 实现指南 (rusl/Rust)

- 自旋锁的本体使用 `AtomicI32` 表示锁变量（`volatile int` 的 Rust 等价物）。
- `__lock()` → `while lock.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() { core::hint::spin_loop(); }`
- `__unlock()` → `lock.store(0, Ordering::Release);`
- `LOCK/UNLOCK` 宏 → Rust trait 方法或闭包模式的 `with_lock()`。
- 必须 `#![no_std]`，不依赖 `std::sync::Mutex` 等标准库锁。