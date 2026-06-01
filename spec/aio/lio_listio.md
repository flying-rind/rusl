# lio_listio.c 规约

> musl libc POSIX 异步 I/O 批量操作实现。一次性发起多个 AIO 请求，支持同步等待（`LIO_WAIT`）和异步通知（`LIO_NOWAIT`）两种模式。

---

## 依赖图

```
lio_listio
  ├─> aio_read / aio_write   (逐个发起 AIO 请求)
  ├─> aio_suspend            (LIO_WAIT 模式下等待所有请求完成)
  ├─> aio_error              (检查操作完成状态)
  ├─> pthread_create → wait_thread
  │     └─> lio_wait → aio_error / aio_suspend
  └─> __syscall(SYS_rt_sigqueueinfo)  或  用户回调
```

---

## 内部类型

### struct lio_state

```c
struct lio_state {
    struct sigevent *sev;
    int cnt;
    struct aiocb *cbs[];
};
```

[Visibility]: Internal — musl AIO 内部结构体，不对外导出

- `sev`: 完成通知事件（SIGEV_SIGNAL / SIGEV_THREAD）
- `cnt`: 请求数量
- `cbs[]`: 灵活数组成员，存储所有 `aiocb` 指针的副本

---

## 内部函数

### 1. lio_wait

```c
static int lio_wait(struct lio_state *st);
```

[Visibility]: Internal — musl AIO 内部轮询等待函数

#### Intent

轮询等待所有 AIO 请求完成。遍历 `st->cbs[]` 检查每个请求的 `aio_error` 状态，使用 `aio_suspend` 等待剩余未完成请求。

#### 系统算法

```
lio_wait(st):
  loop:
    for i in 0..cnt:
      if cbs[i] == NULL: continue
      err = aio_error(cbs[i])
      if err == EINPROGRESS: break
      if err != 0: got_err = 1
      cbs[i] = NULL
    if i == cnt:  // 全部完成
      return got_err ? -1 (errno=EIO) : 0
    aio_suspend(cbs, cnt, 0)  // 等待剩余请求
```

#### 后置条件

- 所有请求已完成：返回 0
- 有请求以非零错误码完成：返回 -1, `errno = EIO`

### 2. notify_signal

```c
static void notify_signal(struct sigevent *sev);
```

[Visibility]: Internal — musl AIO 内部信号通知

#### Intent

通过 `rt_sigqueueinfo` 系统调用发送 AIO 完成信号。

### 3. wait_thread

```c
static void *wait_thread(void *p);
```

[Visibility]: Internal — musl AIO 内部等待线程入口

#### Intent

在独立线程中执行 `lio_wait`，完成后根据 `sev->sigev_notify` 类型发送信号通知或调用用户回调函数。

---

## 函数规约

### lio_listio

```c
int lio_listio(int mode, struct aiocb *restrict const *restrict cbs, int cnt, struct sigevent *restrict sev);
```

[Visibility]: Public — POSIX.1b 实时扩展，定义于 `<aio.h>`

#### Intent

一次性发起 `cnt` 个 AIO 操作请求。每个请求由 `cbs[i]->aio_lio_opcode` 决定操作类型（`LIO_READ` 或 `LIO_WRITE`）。

#### 前置条件

- `cnt >= 0`
- `cbs` 为长度为 `cnt` 的 `aiocb*` 数组（元素可为 NULL，NULL 元素被忽略）
- `cbs[i]->aio_lio_opcode` 为 `LIO_READ` 或 `LIO_WRITE`（其他值被忽略）
- `mode` 为 `LIO_WAIT` 或 `LIO_NOWAIT`
- 若 `sev->sigev_notify != SIGEV_NONE`，需提供有效的 `sigevent`

#### 后置条件

**Case 1: `cnt < 0`**
- `errno = EINVAL`，返回 -1

**Case 2: `mode == LIO_WAIT`**
- 阻塞直到所有操作完成
- 成功：返回 0
- 任一操作失败：`errno = EIO`，返回 -1

**Case 3: `mode == LIO_NOWAIT` 且 `sev` 有效（`SIGEV_NONE` 以外）**
- 立即返回 0
- 创建后台线程等待所有操作完成，完成后发送信号/调用回调

**Case 4: `mode == LIO_NOWAIT` 且 `sev == NULL` 或 `SIGEV_NONE`**
- 发起所有操作后立即返回 0
- 不创建等待线程（无完成通知）

**Case 5: 任一 `aio_read`/`aio_write` 提交失败**
- 释放已分配资源
- `errno = EAGAIN`，返回 -1

#### 系统算法

```
lio_listio(mode, cbs, cnt, sev):
  if cnt < 0: errno = EINVAL, return -1

  if mode == LIO_WAIT || (sev && sev->sigev_notify != SIGEV_NONE):
    分配 lio_state (含 cbs 副本)

  for i in 0..cnt:
    if cbs[i] == NULL: continue
    switch cbs[i]->aio_lio_opcode:
      case LIO_READ:  ret = aio_read(cbs[i]); break
      case LIO_WRITE: ret = aio_write(cbs[i]); break
      default: continue
    if ret != 0: free(st), errno = EAGAIN, return -1

  if mode == LIO_WAIT:
    ret = lio_wait(st); free(st); return ret

  if st != NULL:
    创建 wait_thread 在后台等待完成并通知
  return 0
```

#### 不变量

- 所有非 NULL 且 opcode 为 `LIO_READ`/`LIO_WRITE` 的 `cbs[i]` 都被提交
- `LIO_WAIT` 模式保证返回时所有操作已完成
- 操作顺序不由 `cbs` 数组顺序保证（并发执行）

#### 依赖

- `aio_read` / `aio_write` — 定义于 `src/aio/aio.c`
- `aio_suspend` — 定义于 `src/aio/aio_suspend.c`
- `aio_error` — 定义于 `src/aio/aio.c`
