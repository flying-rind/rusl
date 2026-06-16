# lio_listio.c -- 规约文档

> 源文件: `/home/mangp/桌面/OS/rusl/musl-1.2.6/src/aio/lio_listio.c`
> 对应头文件: `<aio.h>` (由 `musl-1.2.6/include/aio.h` 声明)

---

## 内部数据结构

### struct lio_state

```c
struct lio_state {
    struct sigevent *sev;
    int cnt;
    struct aiocb *cbs[];
};
```

`[Visibility]: Internal (不导出)`

**用途**: 封装异步 `lio_listio` 操作的状态信息，在线程间传递。

**字段说明**:

| 字段 | 类型 | 含义 |
|------|------|------|
| `sev` | `struct sigevent *` | 异步完成通知配置（信号/线程回调） |
| `cnt` | `int` | AIO 控制块列表中的元素个数 |
| `cbs[]` | `struct aiocb *[]` | 柔性数组，存储 AIO 控制块指针的副本（避免用户释放原数组后悬空） |

**Invariant**:
- `cnt >= 0`
- `cbs` 数组长度为 `cnt`，与 `st` 通过 `malloc(sizeof *st + cnt*sizeof *cbs)` 一次性分配
- `sev` 为 `NULL` 或指向有效的 `struct sigevent`

---

## 内部辅助函数

### lio_wait

```c
static int lio_wait(struct lio_state *st);
```

`[Visibility]: Internal (不导出)`

/* Hoare-style Specification */

**Pre-condition**:
- `st != NULL`，指向已初始化的 `struct lio_state`
- `st->cnt >= 0`
- `st->cbs[]` 中每个非 NULL 元素指向一个已提交的 `struct aiocb`（即已完成 `aio_read`/`aio_write` 提交）

**Post-condition**:
- Case 1 (所有 I/O 操作成功完成)
  - 返回 `0`
  - `errno` 不变
- Case 2 (至少一个 I/O 操作失败)
  - 返回 `-1`
  - `errno = EIO`
- Case 3 (`aio_suspend` 被信号中断或失败)
  - 返回 `-1`
  - `errno` 由 `aio_suspend` 设置

**Intent**: 阻塞等待 `st->cbs` 列表中的所有异步 I/O 操作完成，收集各操作是否出错。只要有一个操作出错，整体返回失败。

**System Algorithm**:
1. 初始化 `got_err = 0`。
2. 进入无限循环：
   a. 遍历 `cbs[0..cnt-1]`：
      - 若 `cbs[i]` 为 NULL（已处理完毕），跳过。
      - 调用 `aio_error(cbs[i])` 获取操作状态。
      - 若返回 `EINPROGRESS`（仍在进行中），跳出内层循环（需要继续等待）。
      - 若返回非零错误码，置 `got_err = 1`。
      - 将 `cbs[i]` 置为 NULL（标记已处理）。
   b. 若 `i == cnt`（所有项已处理完毕）：
      - 若 `got_err` 非零，置 `errno = EIO` 并返回 `-1`。
      - 否则返回 `0`。
   c. 否则，调用 `aio_suspend((void *)cbs, cnt, 0)` 阻塞等待。
      - 若 `aio_suspend` 返回 `-1`，直接返回 `-1`（将 `errno` 向上传播）。

**依赖**:
- `aio_error()` — 本模块（aio），定义于 `aio.c`
- `aio_suspend()` — 本模块（aio），定义于 `aio_suspend.c`
- `errno`, `EIO`, `EINPROGRESS` — `<errno.h>`

---

### notify_signal

```c
static void notify_signal(struct sigevent *sev);
```

`[Visibility]: Internal (不导出)`

/* Hoare-style Specification */

**Pre-condition**:
- `sev != NULL`，指向已初始化的 `struct sigevent`
- `sev->sigev_signo` 为有效的实时信号编号
- `sev->sigev_value` 为有效的 `sigval` 值

**Post-condition**:
- 通过 `SYS_rt_sigqueueinfo` 系统调用向当前进程发送实时信号
- `siginfo_t` 的 `si_code` 设置为 `SI_ASYNCIO`，表示异步 I/O 完成
- 调用方线程/进程不会阻塞（信号排队而非等待递送）
- 函数无返回值（`void`）

**Intent**: 向进程自身投递一个异步 I/O 完成信号，携带预设的 `sigev_value`，供用户注册的信号处理器使用。

**System Algorithm**:
1. 构造 `siginfo_t si`，填充：
   - `si_signo = sev->sigev_signo`（信号编号）
   - `si_value = sev->sigev_value`（随信号传递的值）
   - `si_code = SI_ASYNCIO`（标记为异步 I/O 完成）
   - `si_pid = getpid()`（发送进程 PID）
   - `si_uid = getuid()`（发送进程 UID）
2. 调用 `__syscall(SYS_rt_sigqueueinfo, si.si_pid, si.si_signo, &si)` 将信号入队。

**依赖**:
- `getpid()`, `getuid()` — `<unistd.h>`（外部模块: unistd）
- `__syscall`, `SYS_rt_sigqueueinfo` — `"syscall.h"`（内部模块: internal/syscall）
- `siginfo_t`, `SI_ASYNCIO` — `<signal.h>`

---

### wait_thread

```c
static void *wait_thread(void *p);
```

`[Visibility]: Internal (不导出)`

/* Hoare-style Specification */

**Pre-condition**:
- `p != NULL`，指向动态分配的 `struct lio_state`（需确保在线程创建时 `p` 的生存期有效）
- `st->sev` 为 NULL 或指向有效的 `struct sigevent`，且 `sigev_notify` 为 `SIGEV_SIGNAL` 或 `SIGEV_THREAD`
- 本函数作为 `pthread_create` 的线程入口被调用

**Post-condition**:
- 调用 `lio_wait(p)` 阻塞等待所有 I/O 完成
- 调用 `free(p)` 释放 `struct lio_state` 内存
- Case 1 (`sev->sigev_notify == SIGEV_SIGNAL`)
  - 调用 `notify_signal(sev)` 向进程发送完成信号
- Case 2 (`sev->sigev_notify == SIGEV_THREAD`)
  - 调用 `sev->sigev_notify_function(sev->sigev_value)` 执行用户回调
- 返回 `NULL`（`void *` 形式的 `0`）
- 线程以分离状态运行（由调用方在创建时设置 `PTHREAD_CREATE_DETACHED`），退出后自动回收资源

**Intent**: 作为异步 `lio_listio` 的工作线程入口，等待所有 I/O 完成后，根据 `sigev_notify` 配置执行相应通知——发送信号或调用用户回调函数。

**System Algorithm**:
1. 从参数 `p` 中提取 `struct lio_state *st` 和 `struct sigevent *sev = st->sev`。
2. 调用 `lio_wait(st)` 阻塞等待所有 I/O 完成。
3. 调用 `free(st)` 释放 `lio_state` 内存。
4. 根据 `sev->sigev_notify` 分支：
   - `SIGEV_SIGNAL`: 调用 `notify_signal(sev)`。
   - `SIGEV_THREAD`: 调用 `sev->sigev_notify_function(sev->sigev_value)`。
5. 返回 `0`。

**依赖**:
- `lio_wait()` — 本文件内部（static）
- `notify_signal()` — 本文件内部（static）
- `free()` — `<stdlib.h>`（外部模块: stdlib）
- `SIGEV_SIGNAL`, `SIGEV_THREAD` — `<signal.h>`

---

## 对外导出函数

### lio_listio

```c
int lio_listio(int mode, struct aiocb *restrict const *restrict cbs, int cnt, struct sigevent *restrict sev);
```

`[Visibility]: User` — 由 `<aio.h>` 声明，符合 POSIX.1-2001 / POSIX.1-2008 标准

/* Hoare-style Specification */

**Pre-condition**:
- `mode` 为 `LIO_WAIT` 或 `LIO_NOWAIT`
- `cbs` 为指向 `cnt` 个 `struct aiocb *` 指针的数组（若 `cnt == 0` 则为空操作）
- `cnt >= 0`，否则调用失败返回 `-1` 并设 `errno = EINVAL`
- `cbs[i]` 若为非 NULL，其 `aio_lio_opcode` 须为 `LIO_READ`、`LIO_WRITE` 或 `LIO_NOP` 之一
- `sev` 若为非 NULL（仅 `mode == LIO_NOWAIT` 时有意义），其 `sigev_notify` 须为 `SIGEV_NONE`、`SIGEV_SIGNAL` 或 `SIGEV_THREAD` 之一
- `cbs` 数组中的 `aiocb` 对象在操作完成前不得被用户修改

**Post-condition**:
- Case 1 (同步模式 `mode == LIO_WAIT`，所有 I/O 成功)
  - 阻塞直到所有操作完成
  - 返回 `0`
- Case 2 (同步模式 `mode == LIO_WAIT`，至少一个 I/O 失败)
  - 返回 `-1`，`errno = EIO`
- Case 3 (异步模式 `mode == LIO_NOWAIT`，`sev == NULL` 或 `sigev_notify == SIGEV_NONE`)
  - 立即返回 `0`（I/O 操作已提交至后台线程）
  - 无完成通知
- Case 4 (异步模式 `mode == LIO_NOWAIT`，`sev` 有效且 `sigev_notify != SIGEV_NONE`)
  - 立即返回 `0`
  - 创建分离线程（detached thread）后台等待所有 I/O 完成
  - 完成后根据 `sigev_notify` 发送信号或调用回调函数
- Case 5 (内存分配失败)
  - 返回 `-1`，`errno = EAGAIN`
- Case 6 (任何 `aio_read`/`aio_write` 提交失败)
  - 返回 `-1`，`errno = EAGAIN`
  - 已分配的内存被释放
- Case 7 (线程创建失败)
  - 返回 `-1`，`errno = EAGAIN`
  - 已分配的内存被释放
- Case 8 (参数错误)
  - `cnt < 0`: 返回 `-1`，`errno = EINVAL`

**Invariant**:
- 对每个 `cbs[i]`，其 `aio_lio_opcode` 在提交后保持不变
- 异步模式下，若返回 `0` 且 `sev` 有效，则通知必然在线程完成 I/O 等待后触发
- `LIO_NOP` 类型的操作被跳过，不提交也不等待

**Intent**: 批量发起 I/O 操作列表，支持同步阻塞等待和异步通知两种模式。异步模式下通过内部线程池机制实现完成通知，不依赖内核 AIO 子系统。

**System Algorithm**:
1. 参数校验：
   - 若 `cnt < 0`，设 `errno = EINVAL`，返回 `-1`。
2. 分配 `lio_state`（仅在需要等待或通知时）：
   - 若 `mode == LIO_WAIT` 或 `(sev && sev->sigev_notify != SIGEV_NONE)`：
     - 调用 `malloc(sizeof *st + cnt * sizeof *cbs)` 分配 `struct lio_state`。
     - 若分配失败，设 `errno = EAGAIN`，返回 `-1`。
     - 设置 `st->cnt = cnt`，`st->sev = sev`。
     - 调用 `memcpy` 将 `cbs` 数组拷贝到 `st->cbs` 中（防止用户提前释放原数组）。
3. 遍历提交各 I/O 操作：
   - 对 `i` 从 0 到 `cnt-1`：
     - 若 `cbs[i]` 为 NULL，跳过。
     - 根据 `cbs[i]->aio_lio_opcode` 分发：
       - `LIO_READ`: 调用 `aio_read(cbs[i])`。
       - `LIO_WRITE`: 调用 `aio_write(cbs[i])`。
       - 其他（含 `LIO_NOP`）: 跳过。
     - 若提交失败（返回非零），`free(st)`，设 `errno = EAGAIN`，返回 `-1`。
4. 同步等待路径（`mode == LIO_WAIT`）：
   - 调用 `ret = lio_wait(st)`。
   - `free(st)`。
   - 返回 `ret`。
5. 异步通知路径（`st != NULL`，即 `mode == LIO_NOWAIT` 且需要通知）：
   - 初始化 `pthread_attr_t a`：
     - 若 `sigev_notify == SIGEV_THREAD`：使用用户提供的属性或默认属性。
     - 否则：设置栈大小为 `PAGE_SIZE`，保护页大小为 0（最小化资源占用）。
   - 设置 `pthread_attr_setdetachstate(&a, PTHREAD_CREATE_DETACHED)`。
   - 阻塞所有信号（`sigfillset` + `pthread_sigmask(SIG_BLOCK, ...)`），确保子线程继承阻塞的信号集。
   - 调用 `pthread_create(&td, &a, wait_thread, st)` 创建线程：
     - 若失败，`free(st)`，恢复信号掩码，设 `errno = EAGAIN`，返回 `-1`。
     - 若成功，恢复信号掩码（`SIG_SETMASK`）。
6. 返回 `0`。

**依赖**:
- 本模块（aio）:
  - `aio_read()` — 定义于 `aio.c`
  - `aio_write()` — 定义于 `aio.c`
- 本文件内部:
  - `lio_wait()` (static)
  - `wait_thread()` (static)
- 外部模块:
  - `malloc`, `free` — `<stdlib.h>`（来自 libc-stdlib）
  - `memcpy` — `<string.h>`（来自 libc-string）
  - `pthread_attr_t`, `pthread_attr_init`, `pthread_attr_setstacksize`, `pthread_attr_setguardsize`, `pthread_attr_setdetachstate` — `<pthread.h>`（来自 libc-pthread）
  - `pthread_t`, `pthread_create`, `PTHREAD_CREATE_DETACHED` — `<pthread.h>`（来自 libc-pthread）
  - `sigset_t`, `sigfillset`, `pthread_sigmask`, `SIG_BLOCK`, `SIG_SETMASK` — `<signal.h>`（来自 libc-signal）
  - `PAGE_SIZE` — `<limits.h>`
- 常量:
  - `LIO_WAIT`, `LIO_NOWAIT`, `LIO_READ`, `LIO_WRITE`, `LIO_NOP` — `<aio.h>`
  - `SIGEV_NONE`, `SIGEV_SIGNAL`, `SIGEV_THREAD` — `<signal.h>`
  - `EINVAL`, `EAGAIN`, `EIO` — `<errno.h>`
  - `EINPROGRESS` — `<errno.h>`
