# aio.c 规约

> musl libc POSIX 异步 I/O (AIO) 实现。基于线程池的异步 I/O 接口，为每个 AIO 请求创建独立线程执行 I/O 操作。

---

## 依赖图

```
aio_read / aio_write / aio_fsync
  └─> submit
        ├─> __aio_get_queue  (fd → aio_queue 查找/创建)
        │     ├─> fcntl        (F_GETFD 检查 fd 有效性)
        │     ├─> calloc       (map 各级表分配)
        │     └─> pthread_mutex_init / pthread_cond_init
        ├─> pthread_create  → io_thread_func
        │     ├─> lseek / fcntl   (检测 seekable/append)
        │     ├─> read / write / pread / pwrite / fsync / fdatasync
        │     └─> cleanup
        │           ├─> __wake  (futex 通知)
        │           ├─> pthread_cond_broadcast
        │           ├─> __syscall(SYS_rt_sigqueueinfo)  或 用户回调
        │           └─> __aio_unref_queue → free
        └─> sem_wait (等待工作线程就绪)

aio_cancel
  └─> pthread_cancel + __wait (futex 等待取消完成)

__aio_close
  └─> aio_cancel(fd, 0)

__aio_atfork
  └─> 清理子进程中的 map 表
```

---

## 内部类型

### struct aio_thread

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

[Visibility]: Internal — musl AIO 内部结构体

- `td`: 执行 I/O 的线程 ID
- `cb`: 指向 AIO 控制块
- `next`/`prev`: 队列链表指针
- `running`: 原子标志（1=运行中, -1=带等待者运行中, 0=结束）
- `err`/`ret`: I/O 结果（由 `cleanup` 写入 `cb->__err`/`cb->__ret`）

### struct aio_queue

```c
struct aio_queue {
    int fd, seekable, append, ref, init;
    pthread_mutex_t lock;
    pthread_cond_t cond;
    struct aio_thread *head;
};
```

[Visibility]: Internal — musl AIO 内部结构体

每个有活跃 AIO 操作的文件描述符对应一个 `aio_queue`。引用计数管理：当最后一个引用释放时销毁队列。

### struct aio_args

```c
struct aio_args {
    struct aiocb *cb;
    struct aio_queue *q;
    int op;
    sem_t sem;
};
```

[Visibility]: Internal — 线程间参数传递结构体

`sema` 用于确保 `submit()` 在工作线程完成初始化前不返回。

---

## 全局状态

| 变量 | 类型 | 含义 | Visibility |
|------|------|------|------------|
| `maplock` | `pthread_rwlock_t` | 保护 fd→queue 映射表 | Internal |
| `map` | `struct aio_queue *****` | 5 级 fd→queue 映射表 | Internal |
| `aio_fd_cnt` | `volatile int` | 活跃 AIO fd 计数 | Internal |
| `__aio_fut` | `volatile int` | AIO 全局 futex（用于 aio_suspend 通知） | Internal |
| `io_thread_stack_size` | `static size_t` | I/O 线程栈大小缓存 | Internal |

---

## 函数规约

### 1. aio_read

```c
int aio_read(struct aiocb *cb);
```

[Visibility]: Public — POSIX.1b 实时扩展，定义于 `<aio.h>`

#### Intent

发起异步读操作。提交 `cb` 描述的读请求后立即返回；实际 I/O 由工作线程在后台执行。

#### 前置条件

- `cb` 非 NULL，已正确填充 `aio_fildes`, `aio_buf`, `aio_nbytes`, `aio_offset`, `aio_sigevent`
- `cb->aio_fildes` 为有效的可读文件描述符

#### 后置条件

- 成功：返回 0，`cb->__err = EINPROGRESS`
- 失败：返回 -1，`cb->__err` 设置为对应 errno 值，`cb->__ret = -1`

### 2. aio_write

```c
int aio_write(struct aiocb *cb);
```

[Visibility]: Public — POSIX.1b 实时扩展

类似 `aio_read`，执行异步写操作。

### 3. aio_fsync

```c
int aio_fsync(int op, struct aiocb *cb);
```

[Visibility]: Public — POSIX.1b 实时扩展

#### Intent

发起异步 fsync/fdatasync 操作。`op` 必须为 `O_SYNC` 或 `O_DSYNC`。

#### 前置条件

- `op == O_SYNC || op == O_DSYNC`
- `cb->aio_fildes` 为有效的可同步文件描述符

#### 后置条件

- `op` 无效：`errno = EINVAL`，返回 -1

### 4. aio_return

```c
ssize_t aio_return(struct aiocb *cb);
```

[Visibility]: Public — POSIX.1b 实时扩展

#### Intent

获取已完成 AIO 操作的返回值（读写的字节数，或 fsync 的 0，或错误时的 -1）。

#### 前置条件

- `cb` 指向的 AIO 操作已完成（`aio_error(cb) != EINPROGRESS`）

### 5. aio_error

```c
int aio_error(const struct aiocb *cb);
```

[Visibility]: Public — POSIX.1b 实时扩展

#### Intent

检查 AIO 操作的错误状态。包含 `a_barrier()` 确保读取到最新状态。

#### 后置条件

- 返回 `EINPROGRESS` — 操作仍在进行
- 返回 `0` — 操作成功完成
- 返回其他正值 — 操作失败的错误码
- 返回 `ECANCELED` — 操作被取消

### 6. aio_cancel

```c
int aio_cancel(int fd, struct aiocb *cb);
```

[Visibility]: Public — POSIX.1b 实时扩展

#### Intent

取消指定 fd 上的 AIO 操作。若 `cb` 非 NULL，仅取消该特定请求；否则取消该 fd 上所有未完成请求。

#### 前置条件

- 调用前阻塞所有信号（async-signal-safe 要求）

#### 后置条件

- 返回 `AIO_CANCELED` — 至少一个请求被成功取消
- 返回 `AIO_ALLDONE` — 所有请求已完成
- 返回 `AIO_NOTCANCELED` — 至少一个请求无法取消
- 返回 `-1` — 错误（如 fd 无效），`errno` 设置

#### 系统算法

```
aio_cancel(fd, cb):
  阻塞所有信号
  q = __aio_get_queue(fd, 0)
  若 q == NULL: 返回 -1（若 EBADF）或 AIO_ALLDONE
  遍历 q->head 链表:
    若 cb != NULL && cb != p->cb: 跳过
    原子 CAS p->running: 1 → -1
    若 CAS 成功: pthread_cancel + __wait 等待线程退出
    若 p->err == ECANCELED: ret = AIO_CANCELED
  返回 ret
```

### 7. \_\_aio_close

```c
int __aio_close(int fd);
```

[Visibility]: Internal — musl 内部，由 `close()` 调用

#### Intent

`close()` 的 AIO 钩子。若 `aio_fd_cnt > 0`（存在活跃 AIO fd），取消该 fd 上所有未完成 AIO 操作。

### 8. \_\_aio_atfork

```c
void __aio_atfork(int who);
```

[Visibility]: Internal — musl 内部，由 `fork()` 的 `pthread_atfork` 注册

#### Intent

`fork()` 后的 AIO 状态清理。在子进程中清空所有 AIO 映射表（父进程的工作线程在子进程中不存在）。

---

## 关键设计约束

1. **信号阻塞**: 持有 AIO 锁时必须阻塞所有信号，因为 `aio_cancel` 需要在 `close()` 中可用（async-signal-safe）
2. **引用计数**: `aio_queue` 使用引用计数，最后一个退出的工作线程负责释放队列
3. **futex 通知**: 使用原子操作 + futex（而非 pthread 条件变量）通知 `aio_suspend`/`aio_cancel` 的等待者，以保证 AS 安全
4. **线程每请求一个**: 当前实现为每个 AIO 请求创建一个独立线程（one-to-one 模型）
