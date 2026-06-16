# pthread_key_create.c 规约

> musl libc 线程局部存储（TSD）键管理模块。管理 `PTHREAD_KEYS_MAX`（128）个线程局部存储键的创建、删除、析构函数注册与执行。

---

## 依赖图

```
pthread_key_create (weak_alias)
  └─> __pthread_key_create
        ├─> __pthread_self()       (see pthread_self.c spec)
        ├─> __pthread_rwlock_wrlock()  (see pthread_rwlock.c spec)
        └─> __pthread_rwlock_unlock()  (see pthread_rwlock.c spec)

pthread_key_delete (weak_alias)
  └─> __pthread_key_delete
        ├─> __pthread_self()       (see pthread_self.c spec)
        ├─> __block_app_sigs()     (see sigset.c spec - 外部信号模块)
        ├─> __pthread_rwlock_wrlock()
        ├─> __tl_lock()            (weak alias to dummy_0 / real in pthread_create.c)
        ├─> __tl_unlock()          (weak alias to dummy_0 / real in pthread_create.c)
        ├─> __pthread_rwlock_unlock()
        └─> __restore_sigs()       (see sigset.c spec - 外部信号模块)

__pthread_tsd_run_dtors
  ├─> __pthread_self()
  ├─> __pthread_rwlock_rdlock()
  └─> __pthread_rwlock_unlock()

__pthread_key_atfork
  ├─> __pthread_rwlock_rdlock()
  └─> __pthread_rwlock_unlock()
```

---

## 全局状态

### `__pthread_tsd_size`

```c
volatile size_t __pthread_tsd_size = sizeof(void *) * PTHREAD_KEYS_MAX;
```

[Visibility]: Internal (不导出) — 通过 `pthread_impl.h` 内部声明

#### Intent

记录整个线程的 TSD 数组的总字节大小。其值等于 `sizeof(void*) * PTHREAD_KEYS_MAX`（即 `sizeof(void*) * 128`）。

### `__pthread_tsd_main`

```c
void *__pthread_tsd_main[PTHREAD_KEYS_MAX] = { 0 };
```

[Visibility]: Internal (不导出) — 通过 `pthread_impl.h` 内部声明

#### Intent

主线程的 TSD 数组默认存储。当 `pthread_create` 尚未被调用时，主线程的 `self->tsd` 初始为 NULL，第一次调用 `__pthread_key_create` 时会被设置为指向此数组。

### `keys` (static)

```c
static void (*keys[PTHREAD_KEYS_MAX])(void *);
```

[Visibility]: Internal (不导出) — 文件作用域 static 变量

#### Intent

TSD 键析构函数表。`keys[k]` 记录了键 `k` 对应的析构函数指针。`NULL` 表示该槽位空闲。

### `key_lock` (static)

```c
static pthread_rwlock_t key_lock = PTHREAD_RWLOCK_INITIALIZER;
```

[Visibility]: Internal (不导出) — 文件作用域 static 变量

#### Intent

保护 `keys` 数组和 `next_key` 的读写锁。允许 TSD 析构期间多个线程同时读取（`rdlock`），但键创建/删除操作需独占写锁（`wrlock`）。

### `next_key` (static)

```c
static pthread_key_t next_key;
```

[Visibility]: Internal (不导出) — 文件作用域 static 变量

#### Intent

下次键分配时的起始搜索位置。从上次成功分配的键位 + 1 开始循环搜索下一个空闲槽位。

---

## 函数规约

### 1. nodtor (static)

```c
static void nodtor(void *dummy);
```

[Visibility]: Internal (不导出) — 文件作用域 static 函数

#### Intent

哨兵析构函数，体为空。当用户不提供析构函数（`dtor == NULL`）时，`keys[k]` 被设置为 `nodtor` 而非 `NULL`，以区分"槽位空闲"和"无析构"两种状态。

#### 前置条件

无。

#### 后置条件

无操作，直接返回。

#### 不变量

无。

---

### 2. dummy_0 (static)

```c
static void dummy_0(void);
```

[Visibility]: Internal (不导出) — 文件作用域 static 函数，用作 `__tl_lock` / `__tl_unlock` 的弱别名

#### Intent

空函数占位。通过 `weak_alias(dummy_0, __tl_lock)` 和 `weak_alias(dummy_0, __tl_unlock)`，在多线程尚未初始化时提供这些函数的默认实现。当真正的线程支持被链接后，强符号会覆盖此弱别名。

#### 前置条件

无。

#### 后置条件

无操作，直接返回。

#### 不变量

无。

---

### 3. `__pthread_key_atfork`

```c
void __pthread_key_atfork(int who);
```

[Visibility]: Internal (不导出) — 由 fork 处理机制调用

#### Intent

fork 时对 `key_lock` 的状态管理。`who < 0` 表示 prepare 阶段（子进程侧获取读锁），`who == 0` 表示 parent 阶段（父进程释放锁），`who > 0` 表示 child 阶段（子进程重置锁为初始状态）。

#### 前置条件

- 调用者必须处于 fork 流程中（prepare/parent/child 三个阶段之一）

#### 后置条件

- `who < 0`：获取 `key_lock` 的读锁
- `who == 0`：释放 `key_lock` 的读锁
- `who > 0`：将 `key_lock` 重置为 `PTHREAD_RWLOCK_INITIALIZER`

#### 不变量

`key_lock` 在 fork 后始终回到有效状态。

---

### 4. `__pthread_key_create`

```c
int __pthread_key_create(pthread_key_t *k, void (*dtor)(void *));
```

[Visibility]: Internal (不导出) — `__` 前缀内部函数，是 `pthread_key_create` 的真实实现

#### Intent

分配一个新的 TSD 键，注册其析构函数。若线程尚未分配 TSD 数组则初始化之。

#### 前置条件

- `k != NULL`，指向有效的 `pthread_key_t`
- 若 `dtor == NULL`，内部用 `nodtor` 替代

#### 后置条件

- Case 1 成功：
  - `*k` 被设置为新分配的键值（0 到 `PTHREAD_KEYS_MAX-1` 之间的整数）
  - `keys[*k]` 被设置为 `dtor`（或 `nodtor`）
  - `next_key` 更新为 `*k`（下一次搜索的起始位置）
  - 返回 `0`
- Case 2 所有 `PTHREAD_KEYS_MAX`（128）个键槽位均已占用：
  - 返回 `EAGAIN`

#### 系统算法

```
__pthread_key_create(k, dtor):
  1. self = __pthread_self()
  2. 若 self->tsd == NULL，则 self->tsd = __pthread_tsd_main（主线程初始化）
  3. 若 dtor == NULL，则 dtor = nodtor（哨兵值）
  4. 获取 key_lock 写锁
  5. j = next_key
  6. 循环最多 PTHREAD_KEYS_MAX 次：
     a. 若 keys[j] == NULL → 槽位空闲：
        - keys[j] = dtor
        - next_key = *k = j
        - 释放写锁，返回 0
     b. j = (j + 1) % PTHREAD_KEYS_MAX
  7. 释放写锁，返回 EAGAIN
```

#### 不变量

- `keys` 数组中的 `NULL` 条目数 + 非空条目数 == `PTHREAD_KEYS_MAX`
- `next_key` 始终是有效的键索引（0..127）

#### 依赖

- `__pthread_self()` — 获取调用线程的 `pthread` 结构体
- `__pthread_rwlock_wrlock()` / `__pthread_rwlock_unlock()` — 读写锁操作

---

### 5. `__pthread_key_delete`

```c
int __pthread_key_delete(pthread_key_t k);
```

[Visibility]: Internal (不导出) — `__` 前缀内部函数，是 `pthread_key_delete` 的真实实现

#### Intent

删除键 `k`，清除所有线程的 TSD 值并释放析构函数槽位。

#### 前置条件

- `k` 是先前通过 `pthread_key_create` 成功创建的键（未校验有效性）

#### 后置条件

- 所有线程的 `tsd[k]` 均被设置为 0
- `keys[k]` 被设置为 0（槽位标记为空闲）
- 返回 `0`

#### 系统算法

```
__pthread_key_delete(k):
  1. self = __pthread_self()
  2. 阻塞应用程序信号（__block_app_sigs）
  3. 获取 key_lock 写锁
  4. 获取线程列表锁（__tl_lock）
  5. 遍历线程环形链表（td = self, 依次 td = td->next）：
     每个线程的 tsd[k] = 0
     直到 td 回到 self
  6. 释放线程列表锁（__tl_unlock）
  7. keys[k] = 0（释放析构函数槽位）
  8. 释放 key_lock 写锁
  9. 恢复信号（__restore_sigs）
  10. 返回 0
```

#### 不变量

删除操作完成后，被删键 `k` 相关的所有线程状态已清空，新创建键可以安全复用该槽位。

#### 依赖

- `__pthread_self()`
- `__block_app_sigs()` / `__restore_sigs()` — 信号屏蔽/恢复（外部信号模块）
- `__tl_lock()` / `__tl_unlock()` — 线程列表锁（弱别名，真实现由 pthread_create 提供）
- `__pthread_rwlock_wrlock()` / `__pthread_rwlock_unlock()`

---

### 6. `__pthread_tsd_run_dtors`

```c
void __pthread_tsd_run_dtors(void);
```

[Visibility]: Internal (不导出) — 由线程退出路径调用

#### Intent

在线程退出时运行所有已注册的非空、非哨兵 TSD 析构函数。最多重复执行 `PTHREAD_DESTRUCTOR_ITERATIONS`（4）轮，每轮中遍历所有 128 个键。

#### 前置条件

- 调用者为即将退出的线程
- `self->tsd` 有效

#### 后置条件

- 所有非 NULL 且非 `nodtor` 的析构函数被执行（当前线程对应 TSD 值作为参数）
- 若某一轮中有析构函数设置了新的 `tsd[i] != NULL`，则 `tsd_used` 被置 1，触发下一轮
- 最多执行 4 轮

#### 系统算法

```
__pthread_tsd_run_dtors():
  1. self = __pthread_self()
  2. for j in 0..PTHREAD_DESTRUCTOR_ITERATIONS-1 (最多 4 轮):
     a. 若 !self->tsd_used 则提前退出
     b. 获取 key_lock 读锁
     c. self->tsd_used = 0
     d. for i in 0..PTHREAD_KEYS_MAX-1:
        - val = self->tsd[i]
        - dtor = keys[i]
        - self->tsd[i] = 0（先清零再调用，防止递归/重入）
        - 若 val != NULL 且 dtor != NULL 且 dtor != nodtor:
          释放读锁 → 调用 dtor(val) → 重新获取读锁
     e. 释放读锁
```

#### 不变量

- 每轮迭代前 `tsd[i]` 被清零，确保析构函数不会在同一键上被重复调用
- 在调用析构函数前释放读锁，防止析构函数内部调用 `pthread_setspecific` 或 `pthread_key_create` 时死锁

#### 依赖

- `__pthread_self()`
- `__pthread_rwlock_rdlock()` / `__pthread_rwlock_unlock()`

---

### 7. `pthread_key_create`

```c
int pthread_key_create(pthread_key_t *k, void (*dtor)(void *));
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

`weak_alias(__pthread_key_create, pthread_key_create)`

规约同 `__pthread_key_create`。

---

### 8. `pthread_key_delete`

```c
int pthread_key_delete(pthread_key_t k);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出

`weak_alias(__pthread_key_delete, pthread_key_delete)`

规约同 `__pthread_key_delete`。
