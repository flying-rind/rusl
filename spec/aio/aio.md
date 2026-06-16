# musl/src/aio/aio.c 规约

> 本文档对 `aio.c` 中所有符号生成形式化规约，按拓扑排序组织：先定义内部类型和全局状态，再定义内部辅助函数，最后定义对外导出 API。
>
> 本模块实现了基于线程的 POSIX 异步 I/O（AIO）机制。每个有未完成 AIO 操作的文件描述符对应一个 `aio_queue`，通过四级索引表 `map` 进行 fd 到 queue 的映射。全局 `maplock` 以读写锁方式保护该映射表。AIO 工作线程永久阻塞所有信号。详见源码第 19-48 行的设计注释。

---

## 第一部分：内部类型定义

### 1. `struct aio_thread` — AIO 操作线程描述符

```
[Visibility]: Internal (不导出)
```

```c
struct aio_thread {
    pthread_t td;
    struct aiocb *cb;
    struct aio_thread *next, *prev;
    struct aio_queue *q;
    volatile int running;
    int err, op;
    ssize_t ret;
};
```

**作用**: 将每个 AIO 操作与一个工作线程关联，同时作为队列链表节点。

**字段规约**:
| 字段 | 类型 | 含义 |
|------|------|------|
| `td` | `pthread_t` | 工作线程标识符 |
| `cb` | `struct aiocb *` | 指向关联的 AIO 控制块（提交后不可变） |
| `next`, `prev` | `struct aio_thread *` | 队列双向链表指针 |
| `q` | `struct aio_queue *` | 指向所属队列 |
| `running` | `volatile int` | 运行状态：`1`=运行中，`0`=已结束，`-1`=运行中且有待唤醒者（取消路径） |
| `err` | `int` | 操作完成后的错误码（通过 `running` 同步） |
| `op` | `int` | 操作类型（提交后不可变）：`LIO_READ`/`LIO_WRITE`/`O_SYNC`/`O_DSYNC` |
| `ret` | `ssize_t` | 操作返回值（读写的字节数，或 `-1` 表示错误） |

**同步约束**:
- `next`/`prev`/`head`: 受 `q->lock` 保护
- `op`/`q`/`cb`/`td`: 提交后不可变，无锁访问安全
- `running`: 原子更新
- `err`/`ret`: 通过 `running` 同步（写 `ret`/`err` 在 CAS/swap 操作 `running` 之前发生）

---

### 2. `struct aio_queue` — 每个 fd 的 AIO 操作队列

```
[Visibility]: Internal (不导出)
```

```c
struct aio_queue {
    int fd, seekable, append, ref, init;
    pthread_mutex_t lock;
    pthread_cond_t cond;
    struct aio_thread *head;
};
```

**作用**: 为每个有未完成 AIO 操作的文件描述符维护一个队列，管理所有挂起的操作线程。

**字段规约**:
| 字段 | 类型 | 含义 |
|------|------|------|
| `fd` | `int` | 文件描述符（创建后不可变） |
| `seekable` | `int` | fd 是否可定位（由 `lseek(fd, 0, SEEK_CUR) >= 0` 确定） |
| `append` | `int` | 写操作是否使用追加模式（不可定位或 `O_APPEND` 标志） |
| `ref` | `int` | 引用计数，`>0` |
| `init` | `int` | 是否已初始化 `seekable`/`append`（0=未初始化，1=已初始化） |
| `lock` | `pthread_mutex_t` | 保护本队列所有字段的互斥锁 |
| `cond` | `pthread_cond_t` | 条件变量，用于有序操作的同步（前序写操作完成时广播） |
| `head` | `struct aio_thread *` | 队列链表头指针 |

**不变量**:
- **Invariant 1**: `ref` 严格等于引用此队列的活跃 `aio_thread` 数量（含正在创建/销毁过程中的引用）。`ref >= 1` 当且仅当队列在 `map` 中存在或在被访问中。
- **Invariant 2**: 访问 `aio_queue` 的任何成员（除 `__aio_get_queue` 返回前由调用者临时持有外）必须持有 `q->lock`。
- **Invariant 3**: `seekable`/`append`/`init` 仅在 `init==0` 时由第一个线程写入，此后只读。
- **Invariant 4**: `head` 链表中的每个 `aio_thread` 成员均持有对 `q` 的引用（即该线程向 `ref` 贡献了 1）。

---

### 3. `struct aio_args` — submit() 到 io_thread_func() 的参数传递

```
[Visibility]: Internal (不导出)
```

```c
struct aio_args {
    struct aiocb *cb;
    struct aio_queue *q;
    int op;
    sem_t sem;
};
```

**作用**: 通过 `pthread_create` 将参数从 `submit()` 传递到 `io_thread_func()`。

**字段规约**:
| 字段 | 类型 | 含义 |
|------|------|------|
| `cb` | `struct aiocb *` | AIO 控制块 |
| `q` | `struct aio_queue *` | 对应 fd 的队列 |
| `op` | `int` | 操作类型 |
| `sem` | `sem_t` | 信号量，用于同步：`submit()` 调用 `sem_wait()` 等待线程获取 `q->lock` 后再返回 |

---

## 第二部分：内部全局状态

### 4. 模块级全局变量

```
[Visibility]: Internal (不导出)
```

```c
static pthread_rwlock_t maplock = PTHREAD_RWLOCK_INITIALIZER;
static struct aio_queue *****map;
static volatile int aio_fd_cnt;
volatile int __aio_fut;
static size_t io_thread_stack_size;
```

**各变量规约**:

| 变量 | 类型 | 含义 | 访问约束 |
|------|------|------|----------|
| `maplock` | `pthread_rwlock_t` | 全局读写锁，保护 `map` 和 `aio_fd_cnt` | 读 `map` 需持读锁；修改 `map` 或 `aio_fd_cnt` 需持写锁 |
| `map` | `struct aio_queue *****` | 四级索引表，将 fd 映射到 `aio_queue *`。索引：`a=fd>>24`, `b=fd>>16`, `c=fd>>8`, `d=fd`。各级均为 `calloc` 分配的数组。 | 受 `maplock` 保护 |
| `aio_fd_cnt` | `volatile int` | 有未完成 AIO 操作的 fd 数量（即 map 中非 NULL 的队列数）。更新使用 `a_inc`/`a_dec`。 | 受 `maplock` 写锁保护；读侧可使用 `a_barrier()` 快照 |
| `__aio_fut` | `volatile int` | futex 字，`aio_suspend` 等待者在此 futex 上阻塞。任何操作完成时通过 `cleanup()` 将其置零并唤醒。 | 原子操作 |
| `io_thread_stack_size` | `size_t` | AIO 工作线程栈大小。首次创建队列时从 `AT_MINSIGSTKSZ` 辅助向量计算：`MAX(MINSIGSTKSZ+2048, val+512)`。 | 仅由 `__aio_get_queue()` 在持写锁时初始化，之后只读 |

**不变量**:
- **Invariant 5**: `aio_fd_cnt` 始终等于 map 中非 NULL 的四级叶子节点数量。任何对 `map[a][b][c][d]` 的 NULL 到非 NULL 的转换伴随 `a_inc(&aio_fd_cnt)`，任何非 NULL 到 NULL 的转换伴随 `a_dec(&aio_fd_cnt)`。
- **Invariant 6**: 访问任何 AIO 锁（`maplock` 或 `q->lock`）前必须阻塞所有信号（`SIG_BLOCK`）。AIO 工作线程永久阻塞所有信号。这是必须的，因为 `aio_cancel` 在 `close` 中调用，而 `close` 必须是异步信号安全的。

---

## 第三部分：内部辅助函数

### 5. `__aio_get_queue`

```
[Visibility]: Internal (不导出) — 在 aio_impl.h 中声明为 extern hidden
```

```c
static struct aio_queue *__aio_get_queue(int fd, int need);
```

**Level 3 复杂度** — 核心数据结构管理，需要完整的系统算法描述。

---

**Pre-condition:**
- `fd`: 任意整数
- `need`: `0` 或 `1`
  - `need == 0`: 仅查询模式，不创建新的 queue
  - `need == 1`: 若 queue 不存在则创建
- 调用者无需持有任何锁
- 调用者信号状态任意（函数内部自行阻塞/恢复信号）

**Post-condition:**

Case 1: 成功获取或创建 queue
- 返回值: 指向 `aio_queue` 的指针，其 `lock` 已被调用者持有（`pthread_mutex_lock(&q->lock)` 已成功）
- 若 `need == 1` 且队列不存在: 在 map 中创建新队列，`q->fd = fd`，`q->lock`/`q->cond` 初始化，`aio_fd_cnt` 递增
- 若 `io_thread_stack_size == 0` 且需要创建队列: 调用 `__getauxval(AT_MINSIGSTKSZ)` 计算并设置栈大小
- `maplock` 已释放

Case 2: 失败
- 返回 `NULL`（`q == 0`）
- `errno` 被设置:
  - `EBADF`: `fd < 0` 或 `fcntl(fd, F_GETFD) < 0`（无效的 fd）
- `maplock` 已释放，无锁残留

**Intent**: 获取或创建指定 fd 的 AIO 队列，返回时锁住该队列，调用者负责后续解锁。

**System Algorithm**:
1. 若 `fd < 0`: 设置 `errno = EBADF`，返回 `NULL`。
2. 计算四级索引: `a = fd >> 24`, `b = (unsigned char)(fd >> 16)`, `c = (unsigned char)(fd >> 8)`, `d = (unsigned char)fd`。
3. 获取 `maplock` 读锁。
4. 在 `map` 中查找 `map[a][b][c][d]`：
   - 若找到非 NULL 的 `q`，跳至步骤 9。
   - 若未找到且 `need == 0`，释放 `maplock`，返回 `NULL`。
   - 若未找到且 `need == 1`，继续步骤 5。
5. 释放 `maplock` 读锁。
6. 调用 `fcntl(fd, F_GETFD)` 验证 fd 有效性。若失败，返回 `NULL`。
7. 阻塞所有信号（`pthread_sigmask(SIG_BLOCK, &allmask, &origmask)`），记录 `masked = 1`。
8. 获取 `maplock` 写锁。
9. 若需要且未初始化: 计算 `io_thread_stack_size = MAX(MINSIGSTKSZ+2048, __getauxval(AT_MINSIGSTKSZ)+512)`。
10. 按需逐级分配中间层数组（`calloc`）。任何 `calloc` 失败跳至 `out` 标签。
11. 若叶子 `map[a][b][c][d]` 为空，分配新 `aio_queue`，初始化 `fd`/`lock`/`cond`，调用 `a_inc(&aio_fd_cnt)`。
12. 若 `q` 非 NULL: 获取 `q->lock`（`pthread_mutex_lock(&q->lock)`）。
13. `out` 标签: 释放 `maplock`（当前持有的锁类型）。
14. 若 `masked`: 恢复原始信号掩码（`pthread_sigmask(SIG_SETMASK, &origmask, 0)`）。
15. 返回 `q`。

**依赖项**:
- `maplock`, `map`, `aio_fd_cnt`, `io_thread_stack_size`（模块内全局状态）
- `__getauxval(AT_MINSIGSTKSZ)`（外部，sys/auxv.h）
- `fcntl(fd, F_GETFD)`（外部，unistd.h / POSIX）
- `calloc()`（外部，stdlib.h，通过 `#define calloc __libc_calloc` 重定向）

---

### 6. `__aio_unref_queue`

```
[Visibility]: Internal (不导出) — 在 aio_impl.h 中声明为 extern hidden
```

```c
static void __aio_unref_queue(struct aio_queue *q);
```

**Level 2 复杂度** — 引用计数管理含竞态窗口，需意图描述。

---

**Pre-condition:**
- `q`: 有效的 `aio_queue` 指针，非 NULL
- 调用者必须持有 `q->lock`
- `q->ref >= 1`

**Post-condition:**

Case 1: `q->ref > 1`（仍有其他引用）
- `q->ref` 减 1
- `q->lock` 已释放（通过 `pthread_mutex_unlock`）
- 队列对象保持不变

Case 2: `q->ref == 1`（可能为最后引用）
- `q->lock` 被短暂释放后重新获取（中间窗口允许新引用到达）
- 若重新获取锁后 `q->ref == 1` 确认无新引用:
  - 从 `map[a][b][c][d]` 中移除队列指针（置零）
  - `a_dec(&aio_fd_cnt)`
  - `free(q)` 释放队列内存
- 若重新获取锁后 `q->ref > 1`:
  - `q->ref` 减 1，队列保持存活
- 无论如何，函数返回时 `q->lock` 和 `maplock` 均已释放

**Intent**: 安全地释放对队列的引用。由于不能在持有队列锁的同时获取 `maplock`（锁顺序冲突），采用"释放队列锁 -> 获取 maplock -> 重新获取队列锁 -> 确认引用计数"的乐观策略，处理了竞态条件下其他线程可能在此窗口内增加引用的场景。

**System Algorithm**:
1. 若 `q->ref > 1`: 递减 `q->ref`，释放 `q->lock`，返回。
2. 否则（`q->ref == 1`）:
   a. 释放 `q->lock`。
   b. 获取 `maplock` 写锁。
   c. 重新获取 `q->lock`。
   d. 若 `q->ref == 1`（无新引用）: 计算四级索引，将 `map[a][b][c][d] = 0`，`a_dec(&aio_fd_cnt)`，释放两个锁，`free(q)`。
   e. 否则（有新引用）: `q->ref--`，释放两个锁。

**依赖项**:
- `maplock`, `map`, `aio_fd_cnt`（模块内全局状态）
- `free()`（外部，stdlib.h，通过 `#define free __libc_free` 重定向）

---

### 7. `cleanup`

```
[Visibility]: Internal (不导出) — static 函数
```

```c
static void cleanup(void *ctx);
```

**Level 3 复杂度** — 多路径唤醒机制，需要完整的系统算法描述。

---

**Pre-condition:**
- `ctx`: 指向有效的 `struct aio_thread`，非 NULL
- 通过 `pthread_cleanup_push` 注册，在 `io_thread_func` 退出时被调用（正常完成或取消）
- `at->running` 为 `1`（运行中）或 `-1`（运行中且有待唤醒者）

**Post-condition:**
- `cb->__ret` 已设置为操作的返回值
- `at->running` 已通过 `a_swap` 置为 `0`；若原值为 `-1`，已调用 `__wake(&at->running, -1, 1)` 唤醒取消等待者
- `cb->__err` 已通过 `a_swap` 更新；若原值不是 `EINPROGRESS`，已调用 `__wake(&cb->__err, -1, 1)` 唤醒 `aio_suspend` 等待者
- 若 `__aio_fut != 0`，已通过 `a_swap` 将其置零并调用 `__wake(&__aio_fut, -1, 1)`
- `at` 已从 `q->head` 链表中移除
- 已调用 `pthread_cond_broadcast(&q->cond)` 唤醒所有等待前序操作的线程
- 已调用 `__aio_unref_queue(q)` 释放引用
- 若 `sev.sigev_notify == SIGEV_SIGNAL`: 已通过 `__syscall(SYS_rt_sigqueueinfo, ...)` 发送信号
- 若 `sev.sigev_notify == SIGEV_THREAD`: 已重置 `cancel` 状态并调用通知函数 `sev.sigev_notify_function(sev.sigev_value)`

**Intent**: AIO 操作完成时的统一清理路径，负责：（1）更新 `aiocb` 的状态；（2）通知四种类型的等待者（`aio_cancel`、单 `aiocb` 的 `aio_suspend`、列表中 `aio_suspend`、有序操作线程）；（3）处理 SIGEV 通知（信号 / 线程回调）。

**System Algorithm**:
1. 设置 `cb->__ret = at->ret`。
2. 原子地将 `at->running` 置为 `0`，若原值为 `-1`（表示有等待取消的线程），调用 `__wake(&at->running, -1, 1)` 唤醒取消者。
3. 原子地将 `cb->__err` 更新为 `at->err`，若原值不是 `EINPROGRESS`，调用 `__wake(&cb->__err, -1, 1)` 唤醒 `aio_suspend` 等待者。
4. 若 `__aio_fut` 非零，原子地将其置零并调用 `__wake(&__aio_fut, -1, 1)` 唤醒 `aio_suspend` 列表等待者。
5. 获取 `q->lock`。
6. 从 `q->head` 链表中移除 `at`（调整 `prev->next` / `next->prev` / `head`）。
7. 广播 `q->cond` 条件变量（唤醒等待前序写操作完成的有序操作线程）。
8. 调用 `__aio_unref_queue(q)` 释放队列引用（释放 `q->lock`）。
9. 若 `sev.sigev_notify == SIGEV_SIGNAL`:
   - 构造 `siginfo_t`（`si_signo`, `si_value`, `si_code=SI_ASYNCIO`, `si_pid=getpid()`, `si_uid=getuid()`）
   - 调用 `__syscall(SYS_rt_sigqueueinfo, si_pid, si_signo, &si)` 发送信号
10. 若 `sev.sigev_notify == SIGEV_THREAD`:
    - `a_store(&__pthread_self()->cancel, 0)` 重置取消状态
    - 调用 `sev.sigev_notify_function(sev.sigev_value)` 执行回调

**依赖项**:
- `__aio_fut`（模块内全局状态）
- `__aio_unref_queue()`（模块内，见规约 #6）
- `__wake()`（外部，pthread_impl.h inline 函数，基于 futex）
- `__syscall(SYS_rt_sigqueueinfo)`（外部，系统调用接口）
- `getpid()`, `getuid()`（外部，POSIX）
- `__pthread_self()`, `a_store()`, `a_swap()`（外部，pthread_impl.h / atomic.h）

---

### 8. `io_thread_func`

```
[Visibility]: Internal (不导出) — static 函数
```

```c
static void *io_thread_func(void *ctx);
```

**Level 2 复杂度** — 核心 I/O 执行逻辑含有序操作同步，需意图描述。

---

**Pre-condition:**
- `ctx`: 指向有效的 `struct aio_args`，由 `submit()` 分配并传递
- 线程的信号在 `pthread_create` 前已被 `submit()` 阻塞（`SIG_BLOCK` on all signals）
- `args->sem` 信号量值为 `0`

**Post-condition:**

Case 1: 正常完成
- AIO 操作已执行（`read`/`pread`/`write`/`pwrite`/`fsync`/`fdatasync`）
- `at.ret` 设置为操作的返回值（读取/写入的字节数或 `-1`）
- `at.err` 设置为 `0`（成功）或 `errno`（失败）
- `cleanup()` 已被调用（通过 `pthread_cleanup_pop(1)`），完成所有清理和通知
- 返回 `NULL`（实际返回值无意义，因为线程以 `PTHREAD_CREATE_DETACHED` 创建）

Case 2: 取消（通过 `aio_cancel`)
- `cleanup()` 已被调用（通过 `pthread_cleanup_pop(1)` 或隐式清理）
- 线程终止

**Intent**: AIO 工作线程的主函数。获取 `q->lock` 后立即通过 `sem_post` 通知 `submit()` 可以返回，然后将自身链入队列链表。若为第一个操作线程，初始化 fd 的 `seekable`/`append` 属性。对于非读操作或追加写操作，等待所有前序写操作完成后再执行。最后执行实际的 I/O 系统调用。

**System Algorithm**:
1. 从 `args` 中提取 `cb`, `fd`, `op`, `buf`, `len`, `off`, `q`。
2. 获取 `q->lock`。
3. `sem_post(&args->sem)` — 通知 `submit()` 线程可以安全返回。
4. 初始化 `at`（本地栈上的 `aio_thread`）的各字段：`op`, `running=1`, `ret=-1`, `err=ECANCELED`, `q`, `td`, `cb`。
5. 将 `at` 插入 `q->head` 链表头部。
6. 若 `q->init == 0`（首次访问）:
   - 探测 `seekable` = `lseek(fd, 0, SEEK_CUR) >= 0`
   - `append` = `!seekable || (fcntl(fd, F_GETFL) & O_APPEND)`
   - 设置 `init = 1`
7. 注册清理函数 `pthread_cleanup_push(cleanup, &at)`。
8. **有序操作同步**（仅当 `op != LIO_READ` 且 (`op != LIO_WRITE` 或 `q->append`)）:
   - 遍历 `at.next` 开始的链表，查找是否存在 `op == LIO_WRITE` 的前序线程
   - 若存在，在 `q->cond` 上等待（`pthread_cond_wait`），被 `cleanup()` 广播唤醒后重新检查
   - 循环直到链表中不存在前序写操作
9. 释放 `q->lock`。
10. 根据 `op` 执行对应的 I/O 系统调用:
    - `LIO_WRITE` + `append`: `write(fd, buf, len)`
    - `LIO_WRITE` + 非 `append`: `pwrite(fd, buf, len, off)`
    - `LIO_READ` + 不可定位: `read(fd, buf, len)`
    - `LIO_READ` + 可定位: `pread(fd, buf, len, off)`
    - `O_SYNC`: `fsync(fd)`
    - `O_DSYNC`: `fdatasync(fd)`
11. 设置 `at.ret = ret`, `at.err = (ret < 0 ? errno : 0)`。
12. `pthread_cleanup_pop(1)` — 执行 `cleanup()`。
13. 返回 `NULL`。

**依赖项**:
- `sem_post()`（外部，POSIX 信号量）
- `lseek()`, `read()`, `pread()`, `write()`, `pwrite()`, `fsync()`, `fdatasync()`（外部，POSIX I/O）
- `fcntl(fd, F_GETFL)`（外部，POSIX）
- `cleanup()`（模块内，见规约 #7）
- `__pthread_self()`（外部，pthread_impl.h）
- `pthread_cond_wait()`, `pthread_cleanup_push()`, `pthread_cleanup_pop()`（外部，POSIX 线程）

---

### 9. `submit`

```
[Visibility]: Internal (不导出) — static 函数
```

```c
static int submit(struct aiocb *cb, int op);
```

**Level 2 复杂度** — 线程创建与同步编排，需意图描述。

---

**Pre-condition:**
- `cb`: 指向有效的 `struct aiocb`，其 `aio_fildes` 指向一个有效的文件描述符
- `op`: 操作类型，值为 `LIO_READ`、`LIO_WRITE`、`O_SYNC` 或 `O_DSYNC` 之一
- `cb->aio_sigevent` 已正确初始化

**Post-condition:**

Case 1: 成功提交
- 返回值: `0`
- AIO 工作线程已创建并开始运行
- `cb->__err` 已设置为 `EINPROGRESS`
- 队列 `q->ref` 已递增
- `submit()` 已在 `sem_wait(&args.sem)` 上返回，确认工作线程已获取 `q->lock` 并进入执行

Case 2: 提交失败
- 返回值: `-1`
- `errno` 设置为错误码:
  - `EAGAIN`: 无法获取队列或线程创建失败
  - `EBADF`: 无效的文件描述符
- `cb->__ret` 设置为 `-1`，`cb->__err` 设置为对应的 `errno`
- 若线程创建失败：队列引用已通过 `__aio_unref_queue()` 清理

**Intent**: 统一的 AIO 提交入口。负责获取/创建队列、增加引用计数、配置线程属性、阻塞信号、创建工作线程，并通过信号量与工作线程同步以确保在 `pthread_create` 返回时工作线程已取得队列锁。

**System Algorithm**:
1. 调用 `__aio_get_queue(cb->aio_fildes, 1)` 获取/创建队列。
2. 若队列获取失败:
   - 若 `errno != EBADF`: 设置 `errno = EAGAIN`
   - 设置 `cb->__ret = -1`, `cb->__err = errno`
   - 返回 `-1`
3. `q->ref++`, 释放 `q->lock`。
4. 配置线程属性 `a`:
   - 若 `sigev_notify == SIGEV_THREAD` 且提供了 `sigev_notify_attributes`: 复制用户提供的属性
   - 否则: 初始化默认属性，设置栈大小 `io_thread_stack_size`，禁用手动警戒区（`guardsize=0`）
5. 设置线程为分离状态 `PTHREAD_CREATE_DETACHED`。
6. 阻塞所有信号（`sigfillset(&allmask)`, `pthread_sigmask(SIG_BLOCK, ...)`）。
7. 设置 `cb->__err = EINPROGRESS`。
8. 调用 `pthread_create(&td, &a, io_thread_func, &args)`:
   - 若失败: 重新获取 `q->lock`，调用 `__aio_unref_queue(q)` 清理，设置 `cb->__err = errno = EAGAIN`, `cb->__ret = ret = -1`
9. 恢复原始信号掩码。
10. 若成功（`ret == 0`）：循环调用 `sem_wait(&args.sem)` 直到工作线程发出 `sem_post`，确保线程已完成初始化。
11. 返回 `ret`。

**依赖项**:
- `__aio_get_queue()`（模块内，见规约 #5）
- `__aio_unref_queue()`（模块内，见规约 #6）
- `io_thread_func()`（模块内，见规约 #8）
- `io_thread_stack_size`（模块内全局状态）
- `pthread_create()`, `pthread_attr_*`（外部，POSIX 线程）
- `sem_init()`, `sem_wait()`, `sem_post()`（外部，POSIX 信号量）

---

## 第四部分：对外导出 API

### 10. `aio_read`

```
[Visibility]: User — 被 <aio.h> 声明，POSIX 标准接口
```

```c
int aio_read(struct aiocb *cb);
```

**Pre-condition:**
- `cb`: 指向已填充的有效 `struct aiocb`，关键字段:
  - `aio_fildes`: 一个为读而打开的有效的文件描述符
  - `aio_buf`: 指向有效缓冲区的指针
  - `aio_nbytes`: 要读取的字节数
  - 若 fd 可定位: `aio_offset` 为读取起始偏移
  - `aio_sigevent`: 完成通知方式

**Post-condition:**

Case 1: 成功提交
- 返回值: `0`
- 异步读操作已排队，`cb->__err == EINPROGRESS`
- 操作完成后 `cb->__ret` 将包含实际读取的字节数（或 `-1` 表示错误）

Case 2: 提交失败
- 返回值: `-1`
- `errno` 设置: `EAGAIN`（资源不足）或 `EBADF`（无效 fd）
- `cb->__ret == -1`, `cb->__err == errno`

**Intent**: 发起异步读操作。内部调用 `submit(cb, LIO_READ)`。

**依赖项**:
- `submit()`（模块内，见规约 #9）

---

### 11. `aio_write`

```
[Visibility]: User — 被 <aio.h> 声明，POSIX 标准接口
```

```c
int aio_write(struct aiocb *cb);
```

**Pre-condition:**
- `cb`: 指向已填充的有效 `struct aiocb`，关键字段:
  - `aio_fildes`: 一个为写而打开的有效的文件描述符
  - `aio_buf`: 指向包含待写数据的有效缓冲区的指针
  - `aio_nbytes`: 要写入的字节数
  - 若 fd 可定位且非 `O_APPEND`: `aio_offset` 为写入起始偏移
  - `aio_sigevent`: 完成通知方式

**Post-condition:**

Case 1: 成功提交
- 返回值: `0`
- 异步写操作已排队，`cb->__err == EINPROGRESS`
- 操作完成后 `cb->__ret` 将包含实际写入的字节数（或 `-1` 表示错误）
- 写入位置取决于 fd 属性:
  - 若 fd 不可定位或 `O_APPEND`: 使用 `write()`（追加到文件末尾）
  - 否则: 使用 `pwrite()`（从 `aio_offset` 写入）

Case 2: 提交失败
- 返回值: `-1`
- `errno` 设置: `EAGAIN`（资源不足）或 `EBADF`（无效 fd）
- `cb->__ret == -1`, `cb->__err == errno`

**Intent**: 发起异步写操作。内部调用 `submit(cb, LIO_WRITE)`。

**依赖项**:
- `submit()`（模块内，见规约 #9）

---

### 12. `aio_fsync`

```
[Visibility]: User — 被 <aio.h> 声明，POSIX 标准接口
```

```c
int aio_fsync(int op, struct aiocb *cb);
```

**Pre-condition:**
- `op`: 必须为 `O_SYNC` 或 `O_DSYNC`
- `cb`: 指向有效的 `struct aiocb`，其 `aio_fildes` 指向有效的文件描述符
- `cb->aio_sigevent` 已正确初始化

**Post-condition:**

Case 1: 成功提交（`op == O_SYNC` 或 `op == O_DSYNC`）
- 返回值: `0`
- 异步 fsync/fdatasync 操作已排队，`cb->__err == EINPROGRESS`
- 操作完成后 `cb->__ret` 将包含 `fsync()`/`fdatasync()` 的返回值（`0` 成功，`-1` 失败）

Case 2: 无效参数（`op` 既不是 `O_SYNC` 也不是 `O_DSYNC`）
- 返回值: `-1`
- `errno = EINVAL`

Case 3: 提交失败
- 返回值: `-1`
- `errno` 设置: `EAGAIN`（资源不足）或 `EBADF`（无效 fd）
- `cb->__ret == -1`, `cb->__err == errno`

**Intent**: 发起异步文件同步操作。先验证 `op` 参数，再调用 `submit(cb, op)`。与 `aio_read`/`aio_write` 不同，`cb->aio_offset`/`aio_buf`/`aio_nbytes` 在 fsync 中无意义。

**依赖项**:
- `submit()`（模块内，见规约 #9）

---

### 13. `aio_return`

```
[Visibility]: User — 被 <aio.h> 声明，POSIX 标准接口
```

```c
ssize_t aio_return(struct aiocb *cb);
```

**Pre-condition:**
- `cb`: 指向有效的 `struct aiocb`，其异步操作不应仍在进行中（调用者应在调用前通过 `aio_error()` 确认操作已完成）

**Post-condition:**

Case 1: 操作已完成
- 返回值: 操作的返回值（读取/写入的字节数，或 `fsync` 的结果）
- 若操作失败: 返回值 `-1`，具体错误通过 `aio_error()` 获取

Case 2: 操作仍在进行中（调用者违反前置条件）
- 返回值: 未定义（可能返回不完整/无效的值）

**Intent**: 获取已完成的异步操作的返回值。该函数是简单的一行封装 `return cb->__ret;`，无同步操作。调用者的责任是在调用此函数前通过 `aio_error()` 确认操作已完成。

**依赖项**:
- `struct aiocb.__ret`（类型内部字段）

---

### 14. `aio_error`

```
[Visibility]: User — 被 <aio.h> 声明，POSIX 标准接口
```

```c
int aio_error(const struct aiocb *cb);
```

**Pre-condition:**
- `cb`: 指向有效的 `const struct aiocb`

**Post-condition:**
- 调用 `a_barrier()` 确保内存序（保证看到其他线程对 `cb->__err` 的最近写入）
- 返回值: `cb->__err & 0x7fffffff`
  - `EINPROGRESS`: 操作仍在进行中
  - `0`: 操作成功完成
  - `ECANCELED`: 操作已被取消
  - 其他正值: 操作失败，返回值为 `errno` 错误码
  - 注意: `& 0x7fffffff` 屏蔽了最高位，该位可能被内部使用

**Intent**: 查询异步操作的错误/状态。使用内存屏障确保观察到最新的状态值。

**依赖项**:
- `a_barrier()`（外部，atomic.h / 底层架构原子操作）
- `struct aiocb.__err`（类型内部字段）

---

### 15. `aio_cancel`

```
[Visibility]: User — 被 <aio.h> 声明，POSIX 标准接口
```

```c
int aio_cancel(int fd, struct aiocb *cb);
```

**Level 3 复杂度** — 多目标取消含 CAS 竞态处理，需要完整的系统算法描述。

---

**Pre-condition:**
- `fd`: 一个有效的文件描述符
- `cb`: 若为 `NULL`，取消 fd 上所有未完成的 AIO 操作；若非 NULL，仅取消该特定 `cb` 对应的操作
- 若 `cb` 非 NULL 且 `cb->aio_fildes != fd`: 行为未指定（实现主动报错）

**Post-condition:**

Case 1: 所有相关操作已完成（未被取消）
- 返回值: `AIO_ALLDONE` (2)

Case 2: 至少一个操作被成功取消
- 返回值: `AIO_CANCELED` (0)
- 被取消的操作 `cb->__err` 被设置为 `ECANCELED`

Case 3: `cb` 非 NULL 且 `fd != cb->aio_fildes`（未指定行为）
- 返回值: `-1`
- `errno = EINVAL`

Case 4: 无效的 fd 上无操作
- 返回值: `-1`
- `errno = EBADF`

Case 5: 所有信号已被阻塞（函数内部阻塞），调用结束时恢复

**Intent**: 取消指定 fd 上的一个或所有未完成的 AIO 操作。遍历队列链表，对每个匹配的 `aio_thread`：通过 CAS 将 `running` 从 `1` 切换为 `-1`（表示"运行中且有待取消者"），然后调用 `pthread_cancel` 并等待清理完成。CAS 确保不会重复取消已完成的线程。

**System Algorithm**:
1. 若 `cb` 非 NULL 且 `fd != cb->aio_fildes`: 设置 `errno = EINVAL`，返回 `-1`。
2. 初始化 `ret = AIO_ALLDONE`。
3. 阻塞所有信号（`sigfillset(&allmask)`, `pthread_sigmask(SIG_BLOCK, ...)`）。
4. 调用 `__aio_get_queue(fd, 0)` 获取队列（只读查询）:
   - 若返回 NULL:
     - 若 `errno == EBADF`: `ret = -1`
     - 跳至 `done` 标签
5. 遍历 `q->head` 链表中的每个 `aio_thread *p`:
   - 若 `cb` 非 NULL 且 `cb != p->cb`: 跳过
   - 原子 CAS `p->running` 从 `1` 到 `-1`:
     - 若 CAS 成功: 调用 `pthread_cancel(p->td)` 发送取消请求，然后 `__wait(&p->running, 0, -1, 1)` 等待取消完成
     - 若取消成功后 `p->err == ECANCELED`: `ret = AIO_CANCELED`
6. 释放 `q->lock`。
7. `done` 标签: 恢复信号掩码。
8. 返回 `ret`。

**依赖项**:
- `__aio_get_queue()`（模块内，见规约 #5）
- `pthread_cancel()`（外部，POSIX 线程）
- `__wait()`（外部，pthread_impl.h，基于 futex 的等待）
- `a_cas()`（外部，atomic.h）
- `pthread_sigmask()`（外部，POSIX 信号）

---

### 16. `__aio_close`

```
[Visibility]: Internal (不导出) — 在 aio_impl.h 中声明为 extern hidden，
                 同时通过 weak_alias 在 close.c/__stdio_close.c 中以傀儡定义提供默认无操作版本。
                 当链接了 aio.c 时，此强符号覆盖傀儡定义，激活实际的 AIO 取消逻辑。
```

```c
int __aio_close(int fd);
```

**Pre-condition:**
- `fd`: 一个即将被关闭的有效文件描述符
- 调用者通常为 `close()` 或 `__stdio_close()` 的实现

**Post-condition:**

Case 1: `aio_fd_cnt == 0`（无任何未完成的 AIO）
- 无操作，直接返回 `fd`

Case 2: `aio_fd_cnt != 0`（全局存在未完成的 AIO 操作）
- 调用 `aio_cancel(fd, 0)` 尝试取消该 fd 上的所有 AIO 操作
- 返回 `fd`（无修改）
- 注意: 即使取消操作未能成功取消所有线程，函数仍然返回 `fd`，`close()` 将继续执行

**Intent**: 在关闭文件描述符之前取消其上所有未完成的 AIO 操作。这确保了 close 的异步信号安全需求。函数使用 `a_barrier()` 确保观察到最新的 `aio_fd_cnt` 值。返回值始终为传入的 `fd`，使得调用方可以在链式调用中使用。

**依赖项**:
- `aio_fd_cnt`（模块内全局状态）
- `aio_cancel()`（模块内，见规约 #15）
- `a_barrier()`（外部，atomic.h）

---

### 17. `__aio_atfork`

```
[Visibility]: Internal (不导出) — 在 aio_impl.h 中声明为 extern hidden，
                 同时通过 weak_alias 在 fork.c/_Fork.c 中以傀儡定义提供默认无操作版本。
                 当链接了 aio.c 时，此强符号覆盖傀儡定义。
```

```c
void __aio_atfork(int who);
```

**Pre-condition:**
- `who`:
  - `who < 0`: fork 前准备阶段
  - `who == 0`: fork 后父进程恢复阶段
  - `who > 0`: fork 后子进程清理阶段
- 由 `fork()` 或 `_Fork()` 的 fork handler 机制调用

**Post-condition:**

Case 1: `who < 0`（准备阶段）
- 获取 `maplock` 读锁（阻塞直到所有并发的队列创建/销毁完成）
- 确保 fork 时无并发的 map 修改

Case 2: `who == 0`（父进程恢复）
- 释放 `maplock` 读锁
- 恢复 fork 前状态

Case 3: `who > 0`（子进程清理）
- 清零 `aio_fd_cnt = 0`
- 尝试获取 `maplock` 读锁:
  - 若获取失败（说明父进程中的锁未被正确释放，如 `_Fork` 场景）: 设置 `map = 0`，阻止子进程中任何 AIO 操作
  - 若获取成功: 遍历 `map` 所有四级索引，将每个非空叶子 `map[a][b][c][d]` 置为 `NULL`（丢弃父进程的队列引用）
  - 重新初始化 `maplock`（`pthread_rwlock_init(&maplock, 0)`），而非解锁。原因: 在父进程中可能有多于一个的锁引用，而当前线程不是父进程中的锁持有者

**Intent**: fork handler 的三阶段操作。确保 fork 期间 AIO 全局状态的一致性。在子进程中，所有父进程的 AIO 队列和线程引用都被丢弃（因为子进程不继承线程），map 被清空以便子进程可以独立使用 AIO。

**依赖项**:
- `maplock`, `map`, `aio_fd_cnt`（模块内全局状态）

---

## 第五部分：跨文件依赖说明

本文件中 `static` 标注的 `__aio_get_queue` 和 `__aio_unref_queue` 虽在 `aio_impl.h` 中声明为 `extern hidden`，但实际以 `static` 定义于本文件，因此不跨文件导出。在本模块其他 `.c` 文件（如 `aio_suspend.c`、`lio_listio.c`）中，这些函数可能以 `__aio_get_queue`/`__aio_unref_queue` 为名被 `aio_impl.h` 声明并期望链接到本翻译单元的定义。然而由于此处是 `static` 定义，跨文件调用会触发链接错误——除非那些文件内部另有这些函数的 `static` 副本定义。

实际设计中，`aio_impl.h` 的声明为本文件提供函数原型，而 `static` 确保符号不泄露给链接器。`__aio_fut` 以全局 `volatile int` 导出（非 static），被 `aio_suspend.c` 中的 futex 等待操作直接访问。
