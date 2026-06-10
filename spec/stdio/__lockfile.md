# \_\_lockfile.c 规约

> musl libc 内部同步原语实现。为 `FILE` 结构体提供基于 atomic CAS + futex 的轻量级递归锁，供多线程环境下的 stdio 操作使用。

---

## 依赖图

```
__lockfile
  ├─> __pthread_self()->tid  (see pthread_self.c spec)
  ├─> a_cas                   (atomic.h)
  ├─> __futexwait             (futex.h)
  └─> __wake                  (futex.h)

__unlockfile
  ├─> a_swap                  (atomic.h)
  └─> __wake                  (futex.h)
```

---

## 函数规约

### 1. \_\_lockfile

```c
int __lockfile(FILE *f);
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。由 `FLOCK()` / `FFINALLOCK()` 宏在 stdio 函数入口处调用。

#### Intent

获取 `FILE` 对象的互斥锁。支持同一线程递归获取（通过 `f->lock` 存储持有者线程 ID）。当锁已被其他线程持有时，通过 futex 进入内核等待。

#### 前置条件

- `f`: 非 NULL 的 `FILE*`，其 `lock` 字段已初始化
- 若 `f->lock == -1`，表示单线程模式，调用方不应调用此函数（由 `FLOCK` 宏检查）
- 调用线程的 `__pthread_self()` 返回有效线程控制块

#### 后置条件

**Case 1: 成功获取锁**
- `f->lock` 设置为当前线程的 `tid`（或 `tid | MAYBE_WAITERS`）
- 返回 `1`（表示调用方需要在退出时调用 `__unlockfile`）

**Case 2: 递归获取（同一线程已持有锁）**
- `f->lock` 不变
- 返回 `0`（表示调用方不应调用 `__unlockfile`，由外层持有者负责解锁）

#### 系统算法

```
__lockfile(f):
  owner = f->lock
  tid = __pthread_self()->tid

  /* 1. 递归检测：同一线程已持锁 */
  if (owner & ~MAYBE_WAITERS) == tid:
    return 0                         // 递归获取，无需实际操作

  /* 2. 快速路径：无人持锁时 CAS 获取 */
  owner = a_cas(&f->lock, 0, tid)
  if owner == 0:
    return 1                         // 直接获取成功

  /* 3. 慢速路径：futex 等待 */
  loop:
    owner = a_cas(&f->lock, 0, tid | MAYBE_WAITERS)
    if owner == 0:
      return 1                       // 获取成功（带 MAYBE_WAITERS）
    if (owner & MAYBE_WAITERS) or
       a_cas(&f->lock, owner, owner | MAYBE_WAITERS) == owner:
      __futexwait(&f->lock, owner | MAYBE_WAITERS, 1)  // 等待唤醒
```

#### 不变量

- `f->lock` 的值在无竞争时为 `0` 或 `tid`，在有等待者时为 `tid | MAYBE_WAITERS`
- 同一线程可多次获取但只需最后一次 `__unlockfile`（由调用方的 `__need_unlock` 局部变量控制）

---

### 2. \_\_unlockfile

```c
void __unlockfile(FILE *f);
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。由 `FUNLOCK()` 宏在 stdio 函数出口处调用。

#### Intent

释放 `FILE` 对象的互斥锁。若有等待者（`MAYBE_WAITERS` 标志位被设置），通过 `__wake` 唤醒一个等待线程。

#### 前置条件

- `f`: 非 NULL 的 `FILE*`
- 当前线程持有 `f` 的锁（`f->lock` 的 `tid` 部分等于当前线程 `tid`）
- 仅在 `__lockfile` 返回 `1` 时才应调用此函数

#### 后置条件

- `f->lock` 被原子交换为 `0`
- 若交换前的值包含 `MAYBE_WAITERS` 标志，则调用 `__wake(&f->lock, 1, 1)` 唤醒一个等待者
- 若交换前的值不含 `MAYBE_WAITERS` 标志，仅清锁，不唤醒

#### 系统算法

```
__unlockfile(f):
  old = a_swap(&f->lock, 0)          // 原子将 lock 置零
  if old & MAYBE_WAITERS:
    __wake(&f->lock, 1, 1)          // 有等待者，唤醒一个
```

#### 依赖

- `__pthread_self()` — 获取当前线程控制块（`src/thread/pthread_self.c`）
- `a_cas()` / `a_swap()` — 原子比较交换/原子交换（`src/internal/atomic.h`）
- `__futexwait()` / `__wake()` — futex 等待/唤醒（平台相关 futex 实现）
- `MAYBE_WAITERS` — 等待者标志位常量 `0x40000000`（`stdio_impl.h`）
