# pthread_atfork.c 规约

> musl libc 的 fork 处理函数注册和调用实现。注册在 `fork()` 前后执行的回调函数（prepare/parent/child），维护双向链表，在 `fork` 系统调用封装中被调用。

---

## 依赖图

```
pthread_atfork
  ├─> __libc_malloc(sizeof(struct atfork_funcs))  — see malloc (libc)
  ├─> LOCK(lock)                                  — see internal/lock.h
  └─> UNLOCK(lock)                                — see internal/lock.h

__fork_handler (hidden)
  ├─> LOCK(lock)      — see internal/lock.h (仅 who<0 时获取)
  └─> UNLOCK(lock)    — see internal/lock.h (仅 who>=0 时释放)
```

---

## 内部数据结构规约

### 1. struct atfork_funcs (static 变量)

```c
static struct atfork_funcs {
    void (*prepare)(void);
    void (*parent)(void);
    void (*child)(void);
    struct atfork_funcs *prev, *next;
} *funcs;
```

[Visibility]: Internal (不导出) — static 链表头指针

#### Intent

维护已注册的 fork 处理函数的双向链表。每个节点包含三个回调：`prepare`（fork 前调用）、`parent`（fork 后在父进程中调用）、`child`（fork 后在子进程中调用）。任一回调可为 NULL。

---

### 2. lock (static volatile)

```c
static volatile int lock[1];
```

[Visibility]: Internal (不导出) — static 自旋锁，保护链表 `funcs` 的并发访问

---

## 内部函数规约

### 3. __fork_handler (hidden)

```c
hidden void __fork_handler(int who);
```

[Visibility]: Internal (不导出) — `hidden` 可见性，由 `fork()` 系统调用封装调用

#### Intent

在 fork 操作的不同阶段执行已注册的 atfork 回调函数。`who < 0` 表示 fork 前（prepare 阶段），`who == 0` 表示 fork 后父进程，`who > 0` 表示 fork 后子进程。

#### 前置条件

- 由 `fork()` 实现调用，确保在单线程安全上下文中执行
- `who` 为 -1（prepare）、0（parent）或 1（child）

#### 后置条件

- 若 `funcs == NULL`（无注册回调）：直接返回
- prepare 阶段（`who < 0`）：
  - 获取 `lock` 自旋锁
  - 按注册顺序**正向**遍历链表（`p = p->next`），执行每个非 NULL 的 `p->prepare()`
  - 保持 `lock` 锁定状态直到 fork 完成（防止其他线程在 fork 期间注册新回调）
- parent 阶段（`who == 0`）：
  - 按注册顺序**反向**遍历链表（`p = p->prev`），执行每个非 NULL 的 `p->parent()`
  - 释放 `lock`
- child 阶段（`who > 0`）：
  - 按注册顺序**反向**遍历链表（`p = p->prev`），执行每个非 NULL 的 `p->child()`
  - 释放 `lock`（子进程中的锁是新的，但保持在 fork 前的锁定状态需要释放）

#### 系统算法

```
__fork_handler(who):
  1. if !funcs: return
  2. if who < 0:         // prepare
       LOCK(lock)
       for p = funcs; p; p = p->next:
         if p->prepare: p->prepare()
         funcs = p
  3. else:               // parent (who==0) or child (who!=0)
       for p = funcs; p; p = p->prev:
         if !who && p->parent: p->parent()
         else if who && p->child: p->child()
         funcs = p
       UNLOCK(lock)
```

#### 不变量

- prepare 阶段获取锁、parent/child 阶段释放锁，保证 fork 期间注册操作的原子性
- 遍历方向的对称性：prepare 按注册顺序（正向），parent/child 按反注册顺序（反向），符合资源获取/释放的对称语义

---

## 对外导出函数规约

### 4. pthread_atfork

```c
int pthread_atfork(void (*prepare)(void), void (*parent)(void), void (*child)(void));
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

注册在 fork 之前和之后执行的回调函数。`prepare` 在 fork 前调用（通常用于获取锁），`parent` 在 fork 后父进程中调用（释放锁），`child` 在 fork 后子进程中调用（释放/重置锁）。

#### 前置条件

- `prepare`、`parent`、`child` 中至少一个非 NULL（否则注册无意义，但不报错）
- 至少有足够的堆内存用于分配新节点

#### 后置条件

- Case 1 内存不足（`malloc` 返回 NULL）：返回 `ENOMEM`
- Case 2 成功：
  - 在 `lock` 保护下将新节点插入 `funcs` 链表头部（`funcs = new`）
  - 新节点中的 `prepare`/`parent`/`child` 指针被设置
  - 在后续 `fork()` 调用时，`__fork_handler` 会按顺序调用这些回调
  - 返回 0

#### 系统算法

```
pthread_atfork(prepare, parent, child):
  1. new = __libc_malloc(sizeof(struct atfork_funcs))
  2. if !new: return ENOMEM
  3. LOCK(lock)
  4. new->next = funcs; new->prev = 0
  5. new->prepare = prepare; new->parent = parent; new->child = child
  6. if funcs: funcs->prev = new
  7. funcs = new          // 插入链表头部
  8. UNLOCK(lock)
  9. return 0
```

#### 不变量

- 注册的回调在每次 `fork()` 时都会被执行，直到进程终止
- 链表节点不会被释放（进程生命周期内持久存在）
- 回调执行顺序：prepare 按注册顺序（先进先出），parent/child 按反注册顺序（后进先出）

#### 依赖

- `__libc_malloc()` — libc 内部 malloc，被 `#define malloc __libc_malloc` 重定义，避免循环依赖（因为 `malloc` 本身可能使用锁，进而涉及 atfork）
- `LOCK(lock)` / `UNLOCK(lock)` — 自旋锁（宏，展开为 `__lock`/`__unlock`，见 `internal/lock.h`）
- `ENOMEM` — 错误码（来自 `<errno.h>`）
- `calloc` / `realloc` / `free` 被显式 `#undef`，确保本文件不使用标准分配函数
