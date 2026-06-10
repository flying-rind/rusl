# ftrylockfile.c 规约

> musl libc 文件流非阻塞锁定及内部锁链表管理实现。提供 `ftrylockfile` 公开接口和 `__do_orphaned_stdio_locks`、`__unlist_locked_file`、`__register_locked_file` 三个内部辅助函数。

---

## 依赖图

```
ftrylockfile
  ├─> __pthread_self            (see pthread_self.c spec)
  ├─> a_cas                     (see atomic.h)
  ├─> __register_locked_file    (see ftrylockfile.c spec)
  └─> MAYBE_WAITERS             (see stdio_impl.h)

flockfile
  ├─> ftrylockfile              (see ftrylockfile.c spec)
  ├─> __lockfile                (see __lockfile.c spec)
  ├─> __register_locked_file    (see ftrylockfile.c spec)
  └─> __pthread_self            (see pthread_self.c spec)

funlockfile
  ├─> __unlist_locked_file      (see ftrylockfile.c spec)
  └─> __unlockfile              (see __lockfile.c spec)

__register_locked_file
  └─> 直接操作 f 和 self 链表字段

__unlist_locked_file
  ├─> f->lockcount, f->next_locked, f->prev_locked
  └─> __pthread_self()->stdio_locks

__do_orphaned_stdio_locks
  ├─> __pthread_self()->stdio_locks
  └─> a_store                   (see atomic.h)
```

---

## 数据结构分析

### `FILE` 结构体锁相关字段

| 字段 | 类型 | 用途 |
|------|------|------|
| `f->lock` | `volatile int` | 互斥锁：`0` 未锁定；`> 0` 持有者 tid；最高位 `0x40000000` 为 `MAYBE_WAITERS` |
| `f->lockcount` | `long` | 递归锁定计数（同一线程可多次获取锁） |
| `f->prev_locked` | `FILE *` | 线程持有锁链表的前驱 |
| `f->next_locked` | `FILE *` | 线程持有锁链表的后继 |

### `struct pthread` 锁相关字段

| 字段 | 类型 | 用途 |
|------|------|------|
| `self->tid` | `int` | 线程 ID（用于锁持有者标识） |
| `self->stdio_locks` | `void *` / `FILE *` | 该线程持有的 stdio 锁链表头 |

### 锁协议

- `lock` 值的低 30 位为持有者 `tid`（`owner = f->lock & ~MAYBE_WAITERS`）
- `MAYBE_WAITERS` 标志（`0x40000000`）表示有线程在等待该锁
- 每个线程维护一个 `stdio_locks` 链表（通过 `prev_locked`/`next_locked` 链接），记录该线程当前持有的所有 FILE 锁
- `lockcount` 记录递归层数

---

## 函数规约

### 1. ftrylockfile

```c
int ftrylockfile(FILE *f);
```

[Visibility]: User — POSIX 标准函数，声明于 `<stdio.h>`。用户程序可直接调用。

#### Intent

尝试以非阻塞方式获取文件流的互斥锁。若锁立即可用，获取并注册到线程的持有锁链表；若锁已被其他线程持有，立即返回并报告失败。

不同于阻塞式的 `__lockfile`，`ftrylockfile` 绝不等待——是 `flockfile` 的第一步尝试和所有 `FLOCK` 宏的底层前驱。

#### 前置条件

- `f`: 非 NULL 的 `FILE*`
- `__pthread_self()` 返回有效线程控制块
- 调用时不需要持有锁

#### 后置条件

**Case 1: 成功 — 锁可立即获取**
- `f->lock == 0`（无人持有），通过 `a_cas(&f->lock, 0, tid)` 原子获取成功
- `lockcount` 被设置为 `1`（通过 `__register_locked_file` 初始化），`f` 加入线程的 `stdio_locks` 链表
- 对 `owner < 0` 的场景（负值表示未初始化）：`f->lock` 先被重置为 `0`，再尝试 CAS
- 返回 `0`

**Case 2: 成功 — 递归获取（同一线程已持有锁）**
- `(f->lock & ~MAYBE_WAITERS) == tid`
- `f->lockcount < LONG_MAX`（防止溢出）
- `f->lockcount++`
- 返回 `0`

**Case 3: 失败 — 锁被其他线程持有**
- CAS 失败或 `owner` 为其他线程的 tid
- 返回 `-1`

**Case 4: 失败 — 递归计数溢出**
- 同线程持有，但 `f->lockcount == LONG_MAX`
- 返回 `-1`

**Case 5: 负 owner 的修正路径**
- `owner < 0`: `f->lock` 被重置为 `0`（从 `-1` 初始状态过渡到多线程模式）
- 然后按正常 CAS 路径继续

#### 系统算法

```
ftrylockfile(f):
  self = __pthread_self()
  tid = self->tid
  owner = f->lock

  // 1. 递归检测
  if (owner & ~MAYBE_WAITERS) == tid:
    if f->lockcount == LONG_MAX:   // 溢出检查
      return -1
    f->lockcount++                 // 递归获取
    return 0

  // 2. 负 owner 修正（lock == -1，单线程→多线程过渡）
  if owner < 0:
    f->lock = owner = 0

  // 3. CAS 尝试获取
  if owner or a_cas(&f->lock, 0, tid):
    return -1                      // 锁不可用

  // 4. 首次获取成功，注册到线程持有锁链表
  __register_locked_file(f, self)
  return 0
```

#### 不变量

- 锁持有者 tid 存储在 `f->lock` 的低 30 位
- 成功获取后 `f->lockcount >= 1`
- 成功获取后 `f` 存在于 `self->stdio_locks` 链表中
- `f->lock < 0` 的过渡状态被原子处理

#### 依赖

- `__pthread_self()` — 获取当前线程控制块（`src/thread/pthread_self.c`）
- `a_cas` — 原子比较交换（`src/internal/atomic.h`）
- `__register_locked_file` — 将 FILE 注册到线程持有锁链表（定义于同文件）
- `LONG_MAX` — 递归计数上限（`<limits.h>`）
- `MAYBE_WAITERS` — 等待者标志（`stdio_impl.h`，值 `0x40000000`）

---

### 2. \_\_register_locked_file

```c
void __register_locked_file(FILE *f, pthread_t self);
```

[Visibility]: Internal (hidden) — musl 内部实现，不对外暴露。由 `ftrylockfile` 在首次获锁成功后调用。

#### Intent

将 `f` 加入线程 `self` 的 `stdio_locks` 链表头部。初始化 `lockcount` 为 `1` 并设置链表指针。此链表用于线程退出时自动释放所有 stdio 锁（孤儿锁清理）。

#### 前置条件

- `f`: 非 NULL 的 `FILE*`，刚刚由本线程首次获取锁
- `self`: 当前线程的 `pthread_t`
- `f` 尚不在此线程的 `stdio_locks` 链表中

#### 后置条件

- `f->lockcount = 1`
- `f->prev_locked = 0`（新头部无前驱）
- `f->next_locked = self->stdio_locks`（原表头变为后继）
- 若原表头非 NULL：`原表头->prev_locked = f`
- `self->stdio_locks = f`（新头部）

#### 系统算法

```
__register_locked_file(f, self):
  f->lockcount = 1
  f->prev_locked = 0
  f->next_locked = self->stdio_locks
  if f->next_locked:
    f->next_locked->prev_locked = f
  self->stdio_locks = f
```

#### 依赖

- `pthread_t` — 线程控制块类型（`pthread_impl.h`）
- `FILE` 的 `prev_locked` / `next_locked` / `lockcount` 字段

---

### 3. \_\_unlist_locked_file

```c
void __unlist_locked_file(FILE *f);
```

[Visibility]: Internal (hidden) — musl 内部实现，不对外暴露。由 `funlockfile` 在递归计数归零时调用。

#### Intent

将 `f` 从当前线程的 `stdio_locks` 链表中移除。仅在 `f->lockcount == 1`（即将完全释放锁）时被 `funlockfile` 调用。

#### 前置条件

- `f`: 非 NULL 的 `FILE*`
- `f->lockcount != 0`（已在线程的 `stdio_locks` 链表中）
- 调用方已确定需要从链表中移除 `f`

#### 后置条件

- `f` 从 `self->stdio_locks` 链表中移除（通过调整 `prev_locked` / `next_locked` 指针）

#### 系统算法

```
__unlist_locked_file(f):
  if f->lockcount:                        // 已注册则操作（安全冗余检查）
    if f->next_locked:
      f->next_locked->prev_locked = f->prev_locked
    if f->prev_locked:
      f->prev_locked->next_locked = f->next_locked
    else:                                 // f 是链表头部
      __pthread_self()->stdio_locks = f->next_locked
```

#### 依赖

- `__pthread_self()` — 获取当前线程控制块

---

### 4. \_\_do_orphaned_stdio_locks

```c
void __do_orphaned_stdio_locks(void);
```

[Visibility]: Internal (hidden) — musl 内部实现，不对外暴露。在线程退出时被调用以清理孤儿锁。

#### Intent

遍历当前线程持有的所有 stdio 锁（`self->stdio_locks` 链表），对每个锁执行 `a_store(&f->lock, MAYBE_WAITERS)`，将锁值设为 `MAYBE_WAITERS`（表示原持有者已退出，等待者应被唤醒）。这确保线程异常退出时不会永久阻塞等待同一锁的其他线程。

#### 前置条件

- 当前线程即将退出（由线程清理路径调用）
- `__pthread_self()` 返回有效的当前线程控制块

#### 后置条件

- `self->stdio_locks` 链表中所有 `FILE*` 的 `lock` 字段被原子设置为 `MAYBE_WAITERS`（`0x40000000`）
- 此值表示锁可被任何等待者获取（原持有者 tid 无效）

#### 系统算法

```
__do_orphaned_stdio_locks():
  for f = __pthread_self()->stdio_locks; f; f = f->next_locked:
    a_store(&f->lock, MAYBE_WAITERS)    // 原子写入 0x40000000
```

#### 不变量

- 每个孤儿锁的 `lock` 被设置为 `MAYBE_WAITERS`（非 `0`），确保等待在 futex 上的线程能被正确处理
- 遍历完整的链表，无遗漏

#### 依赖

- `__pthread_self()` — 获取当前线程控制块
- `a_store` — 原子存储（`src/internal/atomic.h`）
- `MAYBE_WAITERS` — 等待者标志（`stdio_impl.h`，值 `0x40000000`）
