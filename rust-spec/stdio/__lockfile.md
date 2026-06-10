# __lockfile / __unlockfile 函数规约

## 复杂度分级: Level 3

> musl libc 内部同步原语实现。为 `FILE` 结构体提供基于 atomic CAS + futex 的轻量级递归锁，供多线程环境下的 stdio 操作使用。

---

## 函数接口

```rust
use core::ffi::c_int;

extern "C" fn __lockfile(f: *mut FILE) -> c_int;
extern "C" fn __unlockfile(f: *mut FILE);
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。由 `FLOCK()` / `FUNLOCK()` 宏在 stdio 函数入口/出口处调用。

> 注意：`__lockfile` 的 `f` 参数在 C 中为 `FILE *`（非 const）。Rust 中使用 `*mut FILE` 以允许修改 `f->lock` 字段。`__unlockfile` 同样使用 `*mut FILE`。

---

## 函数详述

### 1. __lockfile

```rust
extern "C" fn __lockfile(f: *mut FILE) -> c_int;
```

#### 前置/后置条件

**[Pre-condition]:**
- `f`: 非空的 `*mut FILE`，其 `lock` 字段已初始化
- 若 `(*f).lock == -1`，表示单线程模式，调用方不应调用此函数（由 `FLOCK` 宏检查）
- 调用线程的 `__pthread_self()` 返回有效线程控制块

**[Post-condition]:**

**Case 1: 成功获取锁（新获取）**
- `(*f).lock` 设置为当前线程的 `tid`（或 `tid | MAYBE_WAITERS`）
- 返回 `1`（表示调用方需要在退出时调用 `__unlockfile`）

**Case 2: 递归获取（同一线程已持有锁）**
- `(*f).lock` 不变
- 返回 `0`（表示调用方不应调用 `__unlockfile`，由外层持有者负责解锁）

#### 系统算法

```
__lockfile(f):
  owner = (*f).lock
  tid = __pthread_self().tid

  /* 1. 递归检测：同一线程已持锁（owner 的低位去掉 MAYBE_WAITERS 后等于 tid） */
  if (owner & !MAYBE_WAITERS) == tid:
    return 0

  /* 2. 快速路径：无人持锁时 CAS 获取 */
  owner = atomic_cas(&(*f).lock, 0, tid)
  if owner == 0:
    return 1

  /* 3. 慢速路径：futex 等待 */
  loop:
    /* 尝试获取锁（带 MAYBE_WAITERS 标志） */
    owner = atomic_cas(&(*f).lock, 0, tid | MAYBE_WAITERS)
    if owner == 0:
      return 1
    /* 若锁已有等待者标志，或能成功加上等待者标志，则等待 */
    if (owner & MAYBE_WAITERS) != 0
       || atomic_cas(&(*f).lock, owner, owner | MAYBE_WAITERS) == owner:
      futex_wait(&(*f).lock, owner | MAYBE_WAITERS)
```

#### 不变量

**[Invariant]:**
- `(*f).lock` 的值在无竞争时为 `0`（无持有者）或 `tid`（当前线程持有），在有等待者时为 `tid | MAYBE_WAITERS`
- 同一线程可多次获取但只需最后一次 `__unlockfile`（由调用方的 `__need_unlock` 局部变量控制）
- `MAYBE_WAITERS` 标志位（`0x40000000`）保证 tid 值与标志不冲突

---

### 2. __unlockfile

```rust
extern "C" fn __unlockfile(f: *mut FILE);
```

#### 前置/后置条件

**[Pre-condition]:**
- `f`: 非空的 `*mut FILE`
- 当前线程持有 `f` 的锁（`(*f).lock` 的低位 `tid` 部分等于当前线程 `tid`）
- 仅在 `__lockfile` 返回 `1` 时才应调用此函数

**[Post-condition]:**
- `(*f).lock` 被原子交换为 `0`
- 若交换前的值包含 `MAYBE_WAITERS` 标志，则调用 `futex_wake` 唤醒一个等待者
- 若交换前的值不含 `MAYBE_WAITERS` 标志，仅清锁，不唤醒

#### 系统算法

```
__unlockfile(f):
  old = atomic_swap(&(*f).lock, 0)    // 原子地将 lock 置零，获取旧值
  if (old & MAYBE_WAITERS) != 0:
    futex_wake(&(*f).lock, 1)         // 有等待者，唤醒一个
```

---

### 意图

提供 `FILE` 对象的线程安全互斥锁。`__lockfile` 获取锁（支持递归），`__unlockfile` 释放锁（含 futex 唤醒逻辑）。

Rust 侧实现：
- 原子操作使用 `core::sync::atomic` 模块（`AtomicI32::compare_exchange`、`AtomicI32::swap`），以 `Ordering::Acquire`/`Ordering::Release` 提供正确的内存序
- futex 系统调用通过 `syscall!` 宏或平台相关内联汇编实现
- 内部可将锁逻辑封装为安全的 RAII 类型（如 `FileLockGuard`），在 `FLOCK`/`FUNLOCK` 宏展开时使用
- 线程 ID 通过 `__pthread_self()` 获取，返回线程控制块引用
- 注意：Rust 的 `AtomicI32` 要求对齐，`(*f).lock` 字段需要在 `#[repr(C)]` 的 `FILE` 结构体中正确对齐

---

## 依赖图

```
__lockfile
  ├─> __pthread_self()::tid        (see pthread_self spec)
  ├─> core::sync::atomic::AtomicI32  (Rust 核心原子操作)
  └─> futex_wait / futex_wake      (平台相关 futex 实现)

__unlockfile
  ├─> core::sync::atomic::AtomicI32  (Rust 核心原子操作)
  └─> futex_wake                   (平台相关 futex 实现)
```

---

## [RELY]

- `__pthread_self()` — 获取当前线程控制块（`pthread` 模块）
- `core::sync::atomic` — 原子 CAS/swap 操作（Rust 核心库）
- `futex_wait` / `futex_wake` — futex 系统调用封装（平台相关实现）
- 常量: `MAYBE_WAITERS` (`0x40000000`)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn __lockfile(f: *mut FILE) -> c_int;`
  `extern "C" fn __unlockfile(f: *mut FILE);`

本模块保证对外提供上述两个 ABI 兼容的函数符号，行为与原 C 实现完全一致：提供基于 atomic CAS + futex 的 FILE 递归锁，返回值语义（`__lockfile` 返回 `1`/`0`）与 C 侧完全一致。
