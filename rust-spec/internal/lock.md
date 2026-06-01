# lock.rs 规约 (Rust)

> **来源 C spec**: `musl/src/internal/spec/lock.md`
> **对应源文件**: `musl/src/internal/lock.h`
> **复杂度层级**: Level 1 — 简单模块
> **依赖图**: 仅依赖 `core::sync::atomic` 模块

---

## 概述

本模块定义了 rusl 中轻量级自旋锁（spinlock）的接口，用于保护内部共享数据结构的临界区。锁基于 `AtomicI32` 上的原子 CAS 操作实现，所有符号均为 rusl 内部使用。

**不变量 (Invariants)**：
- **I1**: 锁变量只有两种语义状态：0 表示未锁定（unlocked），1 表示已被某一线程持有（locked）。
- **I2**: 任何时刻最多只有一个线程能成功获取同一把锁（互斥性）。
- **I3**: 只有成功获取了锁的线程才能释放该锁。

---

## 类型定义

### `SpinLock` — 自旋锁类型

```rust
// Rust 签名
pub(crate) struct SpinLock {
    inner: core::sync::atomic::AtomicI32,
}
```

[Visibility]: Internal — rusl 内部自旋锁类型，POSIX/C 标准未定义

自旋锁是对 `AtomicI32` 的零成本包装。使用 `AtomicI32` 替代 C 的 `volatile int`，提供精确的内存排序控制。`SpinLock` 通过 `const fn new()` 支持静态初始化（等效于 C 中 `volatile int __lock[1] = {0}`）。

**有效状态**：
- `0` — 未锁定
- `1` — 已锁定

---

## 方法声明

### `SpinLock::lock(&self)`

```rust
// Rust 签名
pub(crate) fn lock(&self)
```

[Visibility]: Internal — rusl 内部自旋锁获取

**意图 (Intent)**：
在高竞争场景下使用原子 CAS 自旋等待，避免系统调用开销。用于保护持有时间极短的临界区（通常仅数条指令）。

**前置条件 (Preconditions)**：
- **P1**: 调用者未持有该锁（禁止同一线程递归加锁）。

**后置条件 (Postconditions)**：
- **Case 1（成功获取）**：
  - **Q1**: 函数返回。
  - **Q2**: 锁被标记为已持有（`inner` 值为 1）。
  - **Q3**: 调用者进入临界区，互斥地访问受该锁保护的任何共享资源。
- **Case 2（竞争）**：
  - 函数不会立即返回，而在内部自旋循环中反复尝试 CAS 操作，直到成功获取锁。此过程可能无界等待。

**系统算法 (System Algorithm)**：
使用原子 compare-and-swap 自旋锁：
```rust
while self.inner.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
    core::hint::spin_loop();
}
```

**注意事项**：
- 不应在持有自旋锁时调用可能阻塞的函数（如 futex 等待、内存分配等），以免造成死锁或长延迟。
- 不保证公平性（FIFO 唤醒），先到不一定先得。

---

### `SpinLock::unlock(&self)`

```rust
// Rust 签名
pub(crate) fn unlock(&self)
```

[Visibility]: Internal — rusl 内部自旋锁释放

**意图 (Intent)**：
原子地将锁变量置零，释放临界区，允许其他等待线程进入。

**前置条件 (Preconditions)**：
- **P1**: 调用者必须已通过 `lock()` 成功获取过该锁。
- **P2**: 锁的持有者与解锁者必须是同一线程（或同一执行上下文）。

**后置条件 (Postconditions)**：
- **Q1**: 锁被原子地设置为 0（未锁定状态）。
- **Q2**: 调用者退出临界区，任何对该锁的并发访问限制解除。
- **Q3**: 函数无返回值。

**系统算法 (System Algorithm)**：
使用原子 store 操作将锁清零：
```rust
self.inner.store(0, Ordering::Release);
```
`Ordering::Release` 确保临界区内的所有内存写入在解锁前全局可见。

---

### `SpinLock::new() -> Self`

```rust
// Rust 签名
pub(crate) const fn new() -> Self
```

[Visibility]: Internal

**意图 (Intent)**：
创建一个处于未锁定状态的自旋锁实例。`const fn` 允许在静态初始化上下文中使用，等效于 C 的 `volatile int __lock[1] = {0}`。

**后置条件 (Postconditions)**：
- 返回的 `SpinLock` 处于未锁定状态（`inner == 0`）。

---

## 与其他 rusl 模块的关系

`SpinLock` 替代了 C 中的以下符号：

| C 符号 | Rust 等价 |
|--------|-----------|
| `volatile int __lock[1]` | `SpinLock` |
| `__lock(ptr)` | `spinlock.lock()` |
| `__unlock(ptr)` | `spinlock.unlock()` |
| `LOCK(x)` 宏 | 直接调用 `x.lock()` |
| `UNLOCK(x)` 宏 | 直接调用 `x.unlock()` |

---

## 跨文件依赖

| 依赖符号 | 来源 | 处理方式 |
|---------|------|---------|
| `core::sync::atomic::AtomicI32` | Rust core 库 | 无需追踪（`#![no_std]` 环境中的标准原子类型） |
| `core::sync::atomic::Ordering` | Rust core 库 | 无需追踪 |
| `core::hint::spin_loop()` | Rust core 库 | 无需追踪 |

---

## RELY / GUARANTEE

```
[RELY]
Rust Core 内建类型与函数:
  core::sync::atomic::AtomicI32     // 依赖1: 原子整数类型
  core::sync::atomic::Ordering      // 依赖2: 内存排序枚举
  core::hint::spin_loop()           // 依赖3: 自旋等待提示

[GUARANTEE]
pub(crate) 接口:
  struct SpinLock                   // 自旋锁类型，支持静态初始化
  fn SpinLock::lock(&self)          // 获取自旋锁
  fn SpinLock::unlock(&self)        // 释放自旋锁
  const fn SpinLock::new() -> Self  // 创建新的未锁定自旋锁
```