# lock_ptc — Rust 接口归约

> rusl 内部 pthread cancel (PTC) 锁包装器。使用全局读写锁保护线程取消操作的关键区间，防止在多线程环境中并发的线程创建/取消引发竞争条件。Rust 实现可使用内部 RwLock 抽象替代 POSIX `pthread_rwlock_t`。

## 原始 C 接口

```c
void __inhibit_ptc(void);
void __acquire_ptc(void);
void __release_ptc(void);
```

[Visibility]: Internal — 被 `pthread_impl.h` 声明为 hidden，仅 musl/rusl 内部使用

---

## Rust 外部 ABI 接口

```rust
pub extern "C" fn __inhibit_ptc();
pub extern "C" fn __acquire_ptc();
pub extern "C" fn __release_ptc();
```

---

## 依赖图

```
__inhibit_ptc
  └─> ptc_lock.write()                (内部 RwLock 写锁定)

__acquire_ptc
  └─> ptc_lock.read()                 (内部 RwLock 读锁定)

__release_ptc
  └─> ptc_lock.unlock()               (内部 RwLock 释放)
```

---

## 内部全局变量

### ptc_lock

```rust
// 文件作用域静态变量，不对外导出
static PTC_LOCK: InternalRwLock = InternalRwLock::new();
```

#### Intent

全局读写锁，用于协调 PTC（pthread cancel）相关操作。Rust 实现中可使用基于 `AtomicI32` + futex 的自定义 `InternalRwLock`，也可在支持 `pthread` 时复用 `pthread_rwlock_t`。

#### 不变量

- PTC_LOCK 始终处于有效状态（已初始化、未销毁）
- 任意时刻至多一个线程持有写锁
- 多个线程可以同时持有读锁

---

## 函数规约

### 1. __inhibit_ptc

```rust
pub extern "C" fn __inhibit_ptc();
```

#### Intent

以写模式获取全局 PTC 读写锁，排他性地禁止线程取消操作。通常在需要原子地创建/销毁线程或进行其他不可被取消中断的操作前调用。写锁确保在关键区间内没有并发线程可以读取 PTC 状态。

#### 前置条件

- 全局 PTC 锁已初始化
- 调用者不持有该锁（无论是读锁还是写锁）

#### 后置条件

- 调用者持有 PTC_LOCK 的写锁
- 所有其他线程对 `__acquire_ptc` / `__release_ptc` 的调用均被阻塞
- 调用者处于"PTC 被禁止"状态

---

### 2. __acquire_ptc

```rust
pub extern "C" fn __acquire_ptc();
```

#### Intent

以读模式获取全局 PTC 读写锁。允许多个线程同时持有读锁，但若有线程持有写锁则阻塞。通常在可能被取消的代码路径中调用，以读取 PTC 状态。

#### 前置条件

- 全局 PTC 锁已初始化
- 调用者不持有该锁的写锁

#### 后置条件

- 调用者持有 PTC_LOCK 的读锁
- 若有线程持有写锁，则阻塞直到写锁被释放

---

### 3. __release_ptc

```rust
pub extern "C" fn __release_ptc();
```

#### Intent

释放由 `__inhibit_ptc` 或 `__acquire_ptc` 获取的全局 PTC 读写锁（无论读锁还是写锁）。

#### 前置条件

- 全局 PTC 锁已初始化
- 调用者持有该锁（读锁或写锁）

#### 后置条件

- 调用者释放持有的锁
- 若该锁是写锁且有待处理的读/写请求，其中一个被唤醒
- 若该锁是读锁且是最后一个读锁，待处理的写请求可能被唤醒

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
use core::sync::atomic::{AtomicI32, Ordering};

struct InternalRwLock {
    state: AtomicI32,   // 锁状态编码: 0=解锁, -1=写锁, n>0=有n个读锁
    waiters: AtomicI32,  // 等待者计数
}

impl InternalRwLock {
    pub(crate) const fn new() -> Self { /* ... */ }
    pub(crate) fn write_lock(&self) { /* 写锁定: 使用 AtomicI32 CAS + futex */ }
    pub(crate) fn read_lock(&self) { /* 读锁定: 使用 AtomicI32 CAS + futex */ }
    pub(crate) fn unlock(&self) { /* 解锁: 释放读锁或写锁 */ }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::sync::atomic::AtomicI32          // 依赖1: 原子 I32 类型
  linux futex syscall                    // 依赖2: futex 等待/唤醒

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __inhibit_ptc();
  pub extern "C" fn __acquire_ptc();
  pub extern "C" fn __release_ptc();
                                         // 本模块保证对外提供与 C ABI 兼容的三个 PTC 锁符号
Internal Interface:
  struct InternalRwLock { ... }
  impl InternalRwLock { pub(crate) fn write_lock(&self); pub(crate) fn read_lock(&self); pub(crate) fn unlock(&self); }
                                         // 内部读写锁，供 crate 内部使用
